# Catalog & Delivery Workflows

## 1. Service Catalog Model

Service catalog items represent the billable services an organization offers.

### Schema: `service_catalog_items`

| Field | Type | Purpose |
|-------|------|---------|
| `id` | UUID | Primary key |
| `org_id` | UUID | Multi-tenant isolation |
| `code` | VARCHAR(50) | Unique within org (e.g., `SVC-NURS-001`) |
| `name` | VARCHAR(255) | Human-readable name |
| `description` | TEXT | Optional description |
| `category` | ENUM | `nursing`, `rehab`, `meals`, `companionship`, `transportation`, `other` |
| `unit_type` | ENUM | `visit`, `hour`, `mile`, `meal`, `session` |
| `default_rate` | DECIMAL(10,2) | Default billing rate per unit |
| `is_active` | BOOLEAN | Soft delete / deactivation |

### Validations
- Code: 1-50 characters, unique per organization
- Category: must be one of the defined ENUM values
- Unit type: must be one of the defined ENUM values
- Default rate: must be non-negative

### API Endpoints
- `GET /api/catalog/?category=nursing&active_only=true` - List with filters
- `GET /api/catalog/:id` - Get single item
- `POST /api/catalog/` - Create item (requires `api.catalog.write`)
- `PUT /api/catalog/:id` - Update item (requires `api.catalog.write`)

---

## 2. Package Rule Types

Packages bundle services with specific billing rules. Each package has one or more rules, each linking a service item to a billing model.

### Rule Types

| Type | Description | Key Fields |
|------|-------------|-----------|
| **per_visit** | Flat rate per delivery | `rate` |
| **hourly** | Rate per hour with quarter-hour increments | `rate`, `min_increment` (default 0.25) |
| **tiered** | Rate varies by volume | `rate` (base), `tier_config` JSON |

### Schema: `package_rule_definitions`

| Field | Type | Purpose |
|-------|------|---------|
| `rule_type` | ENUM | `per_visit`, `hourly`, `tiered` |
| `rate` | DECIMAL(10,2) | Base rate |
| `min_increment` | DECIMAL(5,2) | For hourly: minimum billing increment (0.25) |
| `tier_config` | JSON | For tiered: volume-based rate brackets |
| `max_units_per_delivery` | DECIMAL | Maximum units allowed per single delivery |
| `max_units_per_period` | INT | Maximum units per billing period |

### Tier Configuration Format

```json
[
  {"up_to": 4.0, "rate": 50.00},
  {"up_to": 8.0, "rate": 45.00},
  {"up_to": null, "rate": 40.00}
]
```

Rules: last entry must have `up_to: null` (unbounded). Entries are evaluated in order.

### Rule Validations (Backend-Enforced)
- `rule_type` must be `per_visit`, `hourly`, or `tiered`
- `rate` must be non-negative
- Hourly rules default `min_increment` to 0.25 if not specified
- Tiered rules require non-empty `tier_config`
- Last tier must be unbounded (`up_to: null`)
- `max_units_per_delivery` must be positive if set
- Service item must exist and belong to the same organization

### Implementation Files
- Validation: `backend/src/domain/catalog_types.rs` (validate_package_rule, validate_tier_config)
- Service: `backend/src/application/package_service.rs`
- API: `backend/src/api/packages/mod.rs`

---

## 3. Client Plan Assignment Flow

### Lifecycle

```
Draft -> Active -> Paused -> Active -> Completed
                                    -> Cancelled
```

### Plan Creation
1. Operations Manager creates a plan with client name, date range
2. Client identifier and notes are encrypted at rest (AES-256-GCM)
3. Plan starts in `draft` status

### Package Assignment
1. Manager assigns one or more packages to the plan
2. Each assignment has an effective date and optional end date
3. Assignment validates:
   - Package exists, is active, and belongs to same org
   - Dates are valid

### Data Flow
```
client_plans (1) --< client_plan_packages (N) >-- package_definitions (1)
                                                        |
                                                package_rule_definitions (N) >-- service_catalog_items
```

### API Endpoints
- `GET /api/plans/?status=active` - List plans with status filter
- `POST /api/plans/` - Create plan (requires `api.plans.write`)
- `PUT /api/plans/:id` - Update plan status/dates
- `POST /api/plans/:id/packages` - Assign package (requires `api.plans.write`)
- `GET /api/plans/:id/packages` - List assigned packages

---

## 4. Delivery Capture Validations

### Entry Creation Flow (guided selectors)
1. Coach/Clinician selects an **active plan** from a dropdown (fetched from `GET /plans/?status=active`)
2. Selects a **plan package** from a dependent dropdown scoped to the chosen plan (fetched from `GET /plans/:id/packages`, filtered to active assignments)
3. Selects a **service item** from a dependent dropdown scoped to the chosen package's rules (fetched from `GET /packages/:id`, rules joined with `GET /catalog/?active_only=true` for human-readable labels)
4. Enters date, time range, units, optional mileage
5. Backend validates all inputs before persisting

Each selector disables until its prerequisite is chosen. Changing a parent selector clears all downstream selections to prevent stale IDs. If no packages or service items exist for the current selection, the dropdown displays explicit guidance (e.g., "No packages assigned to this plan"). The form submits the same `plan_id`, `plan_package_id`, and `service_item_id` fields the backend expects — the API contract is unchanged.

### Validation Rules (Backend-Enforced)

| Rule | Implementation |
|------|---------------|
| **Quarter-hour increments** | Hourly services require units in 0.25 multiples. `validate_quarter_hour()` checks `(units * 4.0).fract() < 0.001` |
| **Mileage cap** | Maximum 200 miles per visit. `validate_mileage()` rejects values > 200.0 |
| **Plan must be active** | Checks `client_plans.status = 'active'` |
| **Package assignment active** | Checks `client_plan_packages.status = 'active'` |
| **Service in package** | Verifies `package_rule_definitions` contains the service item for this package |
| **Max units per delivery** | If the rule defines `max_units_per_delivery`, units cannot exceed it |
| **Org isolation** | Plan's `org_id` must match authenticated user's org |
| **Date format** | Must be valid `YYYY-MM-DD` |
| **Time format** | Must be valid `HH:MM` or `HH:MM:SS` |
| **Positive units** | Units must be > 0 |
| **Non-negative mileage** | If provided, must be >= 0 |

### Delivery Status Lifecycle
```
Draft -> Submitted -> Verified (by supervisor) -> Billed
                   -> Rejected
```

- Only users with `action.delivery.verify` can set status to `verified`
- Billed entries cannot be modified

### Sensitive Data
- Delivery notes (`notes_enc`) are encrypted at rest using AES-256-GCM
- Notes are never exposed in raw form in audit logs

### API Endpoints
- `GET /api/delivery/?plan_id=&status=&limit=50&offset=0` - List with filters
- `GET /api/delivery/:id` - Get single entry
- `POST /api/delivery/` - Create entry (requires `api.delivery.write`)
- `PUT /api/delivery/:id` - Update entry/status
- `GET /api/delivery/:id/notes` - List eligibility notes
- `POST /api/delivery/:id/notes` - Add eligibility note

### Implementation Files
- Validation: `backend/src/domain/catalog_types.rs`
- Service: `backend/src/application/delivery_service.rs`
- API: `backend/src/api/delivery_entries/mod.rs`
- Frontend: `frontend/src/pages/delivery/mod.rs`

---

## 5. Role-Specific Workflows

| Role | Can Do | Cannot Do |
|------|--------|-----------|
| **Operations Manager** | Create/edit catalog items, create packages with rules, create plans, assign packages, verify deliveries | -- |
| **Coach/Clinician** | View plans (read), create delivery entries, add eligibility notes | Create catalog items, manage packages, manage plans |
| **Billing Specialist** | View plans, view deliveries | Create catalog items, create deliveries |
| **Auditor** | None in this phase (read-only audit access) | All write operations |

### Frontend Permission Checks
- Catalog management buttons: `action.catalog.create`, `action.catalog.edit`
- Package management: `action.packages.create`
- Plan creation: `action.plans.create`
- Delivery logging: `action.delivery.log`
- Delivery verification: `action.delivery.verify`

All checks are advisory; backend enforces independently.

---

## 6. Audit Trail

All operational actions emit audit events:

| Action | Constant |
|--------|----------|
| Service item created | `catalog.item.created` |
| Service item updated | `catalog.item.updated` |
| Package created | `package.created` |
| Package updated | `package.updated` |
| Plan created | `plan.created` |
| Plan updated | `plan.updated` |
| Package assigned to plan | `plan.package.assigned` |
| Delivery entry created | `delivery.entry.created` |
| Delivery entry updated | `delivery.entry.updated` |

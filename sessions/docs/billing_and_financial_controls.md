# Billing & Financial Controls

## Overview

The billing engine converts verified service delivery records into auditable financial transactions. The flow is strictly additive: every real money movement is recorded as an immutable `fund_transaction` entry. No billing record is ever deleted or overwritten — corrections are new records that reference the original.

---

## 1. Billing Lifecycle

```
Delivery Entry (verified)
        │
        ▼
   Charge Generated  ──── Charge Adjustment (optional, additive)
        │
        ▼
   Invoice (draft) ──── Line Items (one per charge, immutable snapshot)
        │
        ▼
   Invoice (issued)
        │
        ├──── Recorded Payment ──── Fund Transaction (credit, immutable)
        │           │
        │           ▼
        │     Invoice → paid / partially_paid
        │
        └──── Recorded Refund ──── Fund Transaction (debit, immutable)
                    │
                    ▼
              Net paid recalculated
```

### States

**Charge statuses**

| Status | Meaning |
|--------|---------|
| `pending` | Generated, not yet adjusted or invoiced |
| `adjusted` | Has one or more charge adjustments |
| `invoiced` | Included in an invoice; no further adjustments allowed |
| `voided` | Excluded from billing (manual void) |

**Invoice statuses**

| Status | Meaning | Allowed next states |
|--------|---------|---------------------|
| `draft` | Generated, not yet sent | `issued`, `voided` |
| `issued` | Sent to payer | `paid`, `partially_paid`, `voided` |
| `partially_paid` | Partial payment received | `paid`, `voided` |
| `paid` | Full net payment received | _(terminal)_ |
| `voided` | Cancelled | _(terminal)_ |

---

## 2. Charge Generation

**Endpoint:** `POST /api/billing/charges/generate`

**Permission required:** `action.billing.generate`

The service iterates all verified delivery entries for the given plan that do not already have a charge record (idempotent). For each entry it looks up the matching `package_rule_definitions` row and applies the appropriate computation:

| Rule Type | Computation |
|-----------|------------|
| `per_visit` | `gross = rate` |
| `hourly` | `gross = units × rate` |
| `tiered` | Brackets applied in order; units fill lower brackets first |

**Tiered billing example:**
```json
[
  {"up_to": 4.0, "rate": 50.00},
  {"up_to": 8.0, "rate": 45.00},
  {"up_to": null, "rate": 40.00}
]
```
For 10 hours: `4×$50 + 4×$45 + 2×$40 = $460`

Entries without a matching package rule are skipped (counted in `skipped` response field).

---

## 3. Charge Adjustments

**Endpoint:** `POST /api/billing/charges/:id/adjustments`

**Rules:**
- Amount cannot be zero
- Reason text is required
- Only allowed on charges in `pending` or `adjusted` status (not `invoiced` or `voided`)
- Positive amount increases the charge; negative decreases it
- Adjustment records are immutable once created
- The parent charge's `adjustment_total` and `net_amount` are recomputed after each adjustment

---

## 4. Invoice Generation

**Endpoint:** `POST /api/billing/invoices/generate`

**Permission required:** `action.billing.generate`

**Process:**
1. Collects all charges in `pending` or `adjusted` status, linked to delivery entries whose `delivery_date` falls within `billing_period_start` to `billing_period_end`
2. Computes `subtotal` (sum of gross amounts), `total_adjustments`, and `total_amount` (sum of net amounts)
3. Generates an `invoice_number` of the form `INV-{YYYYMM}-{plan_suffix}`
4. Creates immutable `invoice_line_items` rows (one per charge — snapshot of amounts at time of invoicing)
5. Marks all included charges as `invoiced` and links them to the new invoice

Returns 400 if no pending charges exist for the plan in the given period.

---

## 5. Idempotency Model

**Endpoint:** `POST /api/payments/`

Every payment request must include an `idempotency_key` (caller-supplied, free-form string). The system enforces a **5-minute duplicate rejection window**:

- If the same `idempotency_key` was used for the same organization within the last 5 minutes → **409 Conflict**
- After 5 minutes, the same key is accepted as a new (retry) payment — this handles network retries without double-charging

**Best practice for callers:**
- Use a UUID generated at the time the user clicks "Record Payment"
- Retain the key for retry in case of network failure

The key is not globally unique — it is scoped per organization and per 5-minute window. Two organizations may use the same key without conflict.

**Implementation:** A dedicated `payment_idempotency_keys` table tracks active keys with a `PRIMARY KEY (org_id, idempotency_key)`. The application uses `INSERT … ON DUPLICATE KEY UPDATE` with a 5-minute `created_at` check, which is both race-safe (no SELECT-then-INSERT gap) and time-windowed (keys older than 5 minutes are atomically refreshed). MySQL's affected-rows semantics distinguish fresh/expired keys (proceed) from active keys (reject 409).

---

## 6. Refund Controls

**Endpoint:** `POST /api/payments/refunds`

**Permission required:** `action.payments.refund`

**Mandatory reason code:** Every refund requires a `reason_code` from the `refund_reason_codes` table.

| Code | Label |
|------|-------|
| `BILLING_ERROR` | Billing Error |
| `SERVICE_NOT_REND` | Service Not Rendered |
| `DUPLICATE_CHARGE` | Duplicate Charge |
| `CONTRACT_CHANGE` | Contract/Rate Change |
| `CLIENT_REQUEST` | Client-Requested Adjustment |
| `PARTIAL_SERVICE` | Partial Service Delivered |
| `QUALITY_ISSUE` | Quality Issue |
| `INSURANCE_ADJ` | Insurance/Payer Adjustment |
| `OTHER` | Other (requires detailed notes) |

**Net-paid cap enforcement:**

```
net_paid = sum(recorded_payments) − sum(recorded_refunds_so_far)
refund.amount must be ≤ net_paid
```

If `refund.amount > net_paid`, the request is rejected with **400 Bad Request** and a message specifying the exact cap.

This cap is enforced atomically before inserting the `fund_transaction` entry.

---

## 7. Immutable Fund Transaction Ledger

The `fund_transactions` table is the authoritative financial ledger. Every real money movement — payment received or refund issued — creates exactly one row with:

| Field | Value |
|-------|-------|
| `transaction_type` | `payment` or `refund` |
| `direction` | `credit` (payment in) or `debit` (refund out) |
| `amount` | Always positive |
| `reference_id` | Links to `recorded_payments.id` or `recorded_refunds.id` |
| `actor_id` | User who initiated the transaction |
| `created_at` | Immutable timestamp |

**There are no UPDATE or DELETE endpoints for fund_transactions.** Corrections are made by recording additional payments or refunds that reference the original invoice, leaving the full trail intact.

**Endpoint:** `GET /api/payments/transactions?invoice_id=&limit=&offset=`
**Permission required:** `api.payments.read`

---

## 8. Reconciliation

**Endpoint:** `POST /api/payments/reconciliation`

**Permission required:** `api.billing.read`

A reconciliation run computes a point-in-time financial summary for a given period:

| Field | Computation |
|-------|------------|
| `total_charges` | Sum of `gross_amount` for charges with delivery dates in period (non-voided) |
| `total_adjustments` | Sum of `adjustment_total` for same charges |
| `total_invoiced` | Sum of `total_amount` for invoices created in period (non-voided) |
| `total_paid` | Sum of payments with `payment_date` in period |
| `total_refunded` | Sum of refunds with `refund_date` in period |
| `net_collected` | `total_paid − total_refunded` |
| `outstanding_balance` | `total_invoiced − net_collected` |
| `pending_charge_count` | Charges in `pending` or `adjusted` status |
| `invoiced_charge_count` | Charges in `invoiced` status |
| `paid_invoice_count` | Invoices in `paid` status |

Reconciliation runs are immutable snapshots — they cannot be updated or deleted. New runs can be generated at any time.

---

## 9. API Reference

### Billing (charges + invoices)

| Method | Path | Permission | Purpose |
|--------|------|-----------|---------|
| POST | `/api/billing/charges/generate` | `action.billing.generate` | Generate charges from verified deliveries |
| GET | `/api/billing/charges` | `api.billing.read` | List charges (plan_id, status filters) |
| GET | `/api/billing/charges/:id` | `api.billing.read` | Charge detail with adjustments |
| POST | `/api/billing/charges/:id/adjustments` | `action.billing.generate` | Post charge adjustment |
| POST | `/api/billing/invoices/generate` | `action.billing.generate` | Generate invoice from pending charges |
| GET | `/api/billing/invoices` | `api.billing.read` | List invoices (plan_id, status filters) |
| GET | `/api/billing/invoices/:id` | `api.billing.read` | Invoice detail with line items |
| PUT | `/api/billing/invoices/:id/status` | `action.billing.approve` | Advance invoice status |

### Payments & Refunds

| Method | Path | Permission | Purpose |
|--------|------|-----------|---------|
| GET | `/api/payments/reason-codes` | `api.payments.read` | List refund reason codes |
| POST | `/api/payments/` | `action.payments.record` | Record payment (idempotency enforced) |
| GET | `/api/payments/` | `api.payments.read` | List payments |
| GET | `/api/payments/:id` | `api.payments.read` | Get payment detail |
| POST | `/api/payments/refunds` | `action.payments.refund` | Record refund (cap enforced, reason required) |
| GET | `/api/payments/refunds` | `api.payments.read` | List refunds |
| GET | `/api/payments/refunds/:id` | `api.payments.read` | Get refund detail |
| GET | `/api/payments/transactions` | `api.payments.read` | List fund transactions (immutable ledger) |
| POST | `/api/payments/reconciliation` | `api.billing.read` | Generate reconciliation run |
| GET | `/api/payments/reconciliation` | `api.billing.read` | List reconciliation runs |
| GET | `/api/payments/reconciliation/:id` | `api.billing.read` | Get reconciliation run detail |

---

## 10. Role Access Matrix

| Role | Can Do | Cannot Do |
|------|--------|-----------|
| System Administrator | All billing operations | — |
| Operations Manager | Generate charges and invoices, approve invoices | Record payments, process refunds |
| Billing Specialist | Record payments, process refunds, generate charges and invoices, approve invoices, run reconciliation | — |
| Coach/Clinician | None (read denied) | All billing operations |
| QA Reviewer | None (read denied) | All billing operations |
| Auditor | Read billing/payment data (via `api.billing.read` — not granted by default; admin can assign) | All write operations |

---

## 11. Audit Trail

Every state-changing billing operation emits an audit entry:

| Action | Trigger |
|--------|---------|
| `billing.charge.generated` | Charge creation |
| `billing.charge.adjusted` | Adjustment posted |
| `billing.invoice.generated` | Invoice created |
| `billing.invoice.status_updated` | Invoice status changed |
| `billing.payment.recorded` | Payment recorded |
| `billing.refund.recorded` | Refund recorded |
| `billing.reconciliation.generated` | Reconciliation run created |

All entries are queryable via `GET /api/audit/` with `resource_type=charge`, `invoice`, `payment`, `refund`, or `reconciliation_run`.

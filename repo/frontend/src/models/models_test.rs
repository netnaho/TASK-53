use crate::models::*;

// ---------------------------------------------------------------------------
// UserProfile
// ---------------------------------------------------------------------------

#[test]
fn user_profile_deserializes_from_json() {
    let json = r#"{
        "id": "u-1",
        "org_id": "o-1",
        "department_id": null,
        "username": "admin",
        "email": "admin@example.com",
        "status": "active",
        "roles": ["System Administrator"],
        "permissions": ["menu.dashboard", "api.billing.read"]
    }"#;
    let user: UserProfile = serde_json::from_str(json).expect("deserialize UserProfile");
    assert_eq!(user.id, "u-1");
    assert_eq!(user.username, "admin");
    assert_eq!(user.roles.len(), 1);
    assert_eq!(user.permissions.len(), 2);
    assert!(user.department_id.is_none());
}

#[test]
fn user_profile_default_has_empty_fields() {
    let user = UserProfile::default();
    assert!(user.id.is_empty());
    assert!(user.permissions.is_empty());
    assert!(user.roles.is_empty());
}

// ---------------------------------------------------------------------------
// LoginResponse
// ---------------------------------------------------------------------------

#[test]
fn login_response_roundtrip() {
    let resp = LoginResponse {
        token: "jwt.tok.en".to_string(),
        user: UserProfile {
            id: "u-2".to_string(),
            org_id: "o-2".to_string(),
            department_id: None,
            username: "ops".to_string(),
            email: "ops@example.com".to_string(),
            status: "active".to_string(),
            roles: vec!["Ops Manager".to_string()],
            permissions: vec!["api.catalog.read".to_string()],
        },
    };
    let serialized = serde_json::to_string(&resp).expect("serialize LoginResponse");
    assert!(serialized.contains("jwt.tok.en"));
    let deserialized: LoginResponse =
        serde_json::from_str(&serialized).expect("deserialize LoginResponse");
    assert_eq!(deserialized.token, "jwt.tok.en");
    assert_eq!(deserialized.user.username, "ops");
}

// ---------------------------------------------------------------------------
// PaginatedResponse
// ---------------------------------------------------------------------------

#[test]
fn paginated_response_fields_accessible() {
    let row = UserRow {
        id: "u-3".to_string(),
        org_id: "o-1".to_string(),
        department_id: None,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        status: "active".to_string(),
        created_at: "2024-01-01T00:00:00".to_string(),
        updated_at: "2024-01-01T00:00:00".to_string(),
    };
    let paged = PaginatedResponse {
        data: vec![row],
        total: 1,
        page: 1,
        per_page: 25,
    };
    assert_eq!(paged.total, 1);
    assert_eq!(paged.data.len(), 1);
    assert_eq!(paged.data[0].username, "alice");
}

// ---------------------------------------------------------------------------
// ErrorEnvelope
// ---------------------------------------------------------------------------

#[test]
fn error_envelope_deserializes() {
    let json = r#"{
        "error": {
            "code": "NOT_FOUND",
            "message": "Resource not found",
            "trace_id": "abc123"
        }
    }"#;
    let env: ErrorEnvelope = serde_json::from_str(json).expect("deserialize ErrorEnvelope");
    assert_eq!(env.error.code, "NOT_FOUND");
    assert_eq!(env.error.trace_id, "abc123");
}

// ---------------------------------------------------------------------------
// ServiceItemRow
// ---------------------------------------------------------------------------

#[test]
fn service_item_row_default_has_empty_fields() {
    let svc = ServiceItemRow::default();
    assert!(svc.id.is_empty());
    assert!(svc.code.is_empty());
    assert!(!svc.is_active); // bool default is false
}

#[test]
fn service_item_row_deserializes() {
    let json = r#"{
        "id": "svc-1",
        "org_id": "o-1",
        "code": "NURSING-VISIT",
        "name": "Nursing Visit",
        "description": null,
        "category": "nursing",
        "unit_type": "visit",
        "default_rate": 75.0,
        "is_active": true
    }"#;
    let svc: ServiceItemRow = serde_json::from_str(json).expect("deserialize ServiceItemRow");
    assert_eq!(svc.code, "NURSING-VISIT");
    assert_eq!(svc.default_rate, 75.0);
    assert!(svc.is_active);
}

// ---------------------------------------------------------------------------
// ReconciliationRunRow
// ---------------------------------------------------------------------------

#[test]
fn reconciliation_run_row_default_zero_values() {
    let row = ReconciliationRunRow::default();
    assert!(row.id.is_empty());
    assert_eq!(row.total_charges, 0.0);
    assert_eq!(row.outstanding_balance, 0.0);
    assert_eq!(row.pending_charge_count, 0);
}

// ---------------------------------------------------------------------------
// PackageRow / PackageDetail
// ---------------------------------------------------------------------------

#[test]
fn package_row_default_has_empty_fields() {
    let pkg = PackageRow::default();
    assert!(pkg.id.is_empty());
    assert!(!pkg.is_active);
}

#[test]
fn client_plan_row_roundtrip() {
    let plan = ClientPlanRow {
        id: "pl-1".to_string(),
        org_id: "o-1".to_string(),
        department_id: None,
        project_id: None,
        client_name: "Jane Doe".to_string(),
        status: "active".to_string(),
        start_date: "2024-01-01".to_string(),
        end_date: Some("2024-12-31".to_string()),
        created_by: Some("u-1".to_string()),
    };
    let serialized = serde_json::to_string(&plan).expect("serialize ClientPlanRow");
    let deserialized: ClientPlanRow =
        serde_json::from_str(&serialized).expect("deserialize ClientPlanRow");
    assert_eq!(deserialized.client_name, "Jane Doe");
    assert_eq!(deserialized.status, "active");
}

// ---------------------------------------------------------------------------
// RoleRow
// ---------------------------------------------------------------------------

#[test]
fn role_row_deserializes() {
    let json = r#"{
        "id": "r-1",
        "name": "System Administrator",
        "description": "Full access",
        "is_system": true,
        "created_at": "2024-01-01T00:00:00",
        "updated_at": "2024-01-01T00:00:00"
    }"#;
    let role: RoleRow = serde_json::from_str(json).expect("deserialize RoleRow");
    assert_eq!(role.name, "System Administrator");
    assert!(role.is_system);
}

// ---------------------------------------------------------------------------
// InvoiceRow
// ---------------------------------------------------------------------------

#[test]
fn invoice_row_deserializes() {
    let json = r#"{
        "id": "inv-1",
        "org_id": "o-1",
        "plan_id": "pl-1",
        "invoice_number": "INV-2024-001",
        "billing_period_start": "2024-01-01",
        "billing_period_end": "2024-01-31",
        "subtotal": 450.00,
        "total_adjustments": -25.00,
        "total_amount": 425.00,
        "status": "issued",
        "generated_by": "u-1",
        "notes": null,
        "created_at": "2024-02-01T10:00:00",
        "updated_at": "2024-02-01T10:00:00"
    }"#;
    let inv: InvoiceRow = serde_json::from_str(json).expect("deserialize InvoiceRow");
    assert_eq!(inv.invoice_number, "INV-2024-001");
    assert_eq!(inv.total_amount, 425.00);
    assert_eq!(inv.status, "issued");
    assert!(inv.notes.is_none());
}

#[test]
fn invoice_row_equality_by_id() {
    let inv1 = InvoiceRow {
        id: "inv-1".to_string(),
        org_id: "o-1".to_string(),
        plan_id: "pl-1".to_string(),
        invoice_number: "INV-001".to_string(),
        billing_period_start: "2024-01-01".to_string(),
        billing_period_end: "2024-01-31".to_string(),
        subtotal: 100.0,
        total_adjustments: 0.0,
        total_amount: 100.0,
        status: "issued".to_string(),
        generated_by: "u-1".to_string(),
        notes: None,
        created_at: "2024-02-01".to_string(),
        updated_at: "2024-02-01".to_string(),
    };
    let inv2 = inv1.clone();
    assert_eq!(inv1, inv2);
}

// ---------------------------------------------------------------------------
// ScoringTemplateRow / EvaluationRow
// ---------------------------------------------------------------------------

#[test]
fn scoring_template_row_deserializes() {
    let json = r#"{
        "id": "tmpl-1",
        "org_id": "o-1",
        "name": "Nurse Competency",
        "description": null,
        "rounding_interval": 0.5,
        "max_score": 100.0,
        "is_active": true,
        "created_by": "u-1",
        "created_at": "2024-01-01T00:00:00",
        "updated_at": "2024-01-01T00:00:00"
    }"#;
    let tmpl: ScoringTemplateRow = serde_json::from_str(json).expect("deserialize ScoringTemplateRow");
    assert_eq!(tmpl.name, "Nurse Competency");
    assert_eq!(tmpl.rounding_interval, 0.5);
    assert_eq!(tmpl.max_score, 100.0);
    assert!(tmpl.is_active);
}

#[test]
fn evaluation_row_requires_second_review_flag() {
    let eval = EvaluationRow {
        id: "eval-1".to_string(),
        org_id: "o-1".to_string(),
        delivery_entry_id: "entry-1".to_string(),
        template_id: "tmpl-1".to_string(),
        evaluator_id: "u-1".to_string(),
        status: "second_review_required".to_string(),
        prior_final_score: Some(75.0),
        raw_score: None,
        weighted_score: None,
        final_score: None,
        requires_second_review: true,
        score_delta: Some(15.0),
        second_reviewer_id: None,
        second_reviewed_at: None,
        overall_comment: None,
        created_at: "2024-01-15T10:00:00".to_string(),
        updated_at: "2024-01-15T10:00:00".to_string(),
    };
    assert!(eval.requires_second_review);
    assert_eq!(eval.score_delta, Some(15.0));
    assert_eq!(eval.status, "second_review_required");
}

// ---------------------------------------------------------------------------
// KpiSummary
// ---------------------------------------------------------------------------

#[test]
fn kpi_summary_roundtrip() {
    let kpi = KpiSummary {
        period_start: "2024-01-01".to_string(),
        period_end: "2024-03-31".to_string(),
        attendance_rate_pct: 92.5,
        repurchase_rate_pct: 78.3,
        staff_utilization_pct: 85.0,
        avg_score: Some(88.5),
        second_review_rate_pct: 4.2,
    };
    let serialized = serde_json::to_string(&kpi).expect("serialize KpiSummary");
    let deserialized: KpiSummary =
        serde_json::from_str(&serialized).expect("deserialize KpiSummary");
    assert_eq!(deserialized.attendance_rate_pct, 92.5);
    assert_eq!(deserialized.avg_score, Some(88.5));
    assert_eq!(deserialized.period_start, "2024-01-01");
}

#[test]
fn kpi_summary_avg_score_optional() {
    let kpi = KpiSummary {
        period_start: "2024-01-01".to_string(),
        period_end: "2024-01-31".to_string(),
        attendance_rate_pct: 90.0,
        repurchase_rate_pct: 70.0,
        staff_utilization_pct: 80.0,
        avg_score: None, // no evaluations yet
        second_review_rate_pct: 0.0,
    };
    let json = serde_json::to_string(&kpi).unwrap();
    let de: KpiSummary = serde_json::from_str(&json).unwrap();
    assert!(de.avg_score.is_none());
}

// ---------------------------------------------------------------------------
// DeliveryEntryRow
// ---------------------------------------------------------------------------

#[test]
fn delivery_entry_row_deserializes() {
    let json = r#"{
        "id": "entry-1",
        "org_id": "o-1",
        "plan_id": "pl-1",
        "plan_package_id": "pp-1",
        "service_item_id": "svc-1",
        "provider_id": "u-coach",
        "delivery_date": "2024-03-15",
        "start_time": "09:00",
        "end_time": "11:30",
        "units": 2.5,
        "mileage": 12.0,
        "status": "verified"
    }"#;
    let entry: DeliveryEntryRow = serde_json::from_str(json).expect("deserialize DeliveryEntryRow");
    assert_eq!(entry.units, 2.5);
    assert_eq!(entry.status, "verified");
    assert_eq!(entry.mileage, Some(12.0));
    assert_eq!(entry.start_time.as_deref(), Some("09:00"));
}

// ---------------------------------------------------------------------------
// OrderVolumeRow / RevenueReportRow
// ---------------------------------------------------------------------------

#[test]
fn order_volume_row_deserializes() {
    let json = r#"{
        "period": "2024-W12",
        "delivery_count": 45,
        "unique_plans": 12,
        "unique_providers": 8
    }"#;
    let row: OrderVolumeRow = serde_json::from_str(json).expect("deserialize OrderVolumeRow");
    assert_eq!(row.delivery_count, 45);
    assert_eq!(row.unique_plans, 12);
}

#[test]
fn revenue_report_row_deserializes() {
    let json = r#"{
        "period": "2024-W12",
        "gross_charges": 5000.0,
        "net_charges": 4800.0,
        "total_invoiced": 4800.0,
        "total_paid": 4200.0,
        "total_refunded": 100.0,
        "refund_rate_pct": 2.38
    }"#;
    let row: RevenueReportRow = serde_json::from_str(json).expect("deserialize RevenueReportRow");
    assert_eq!(row.gross_charges, 5000.0);
    assert_eq!(row.total_paid, 4200.0);
    assert!((row.refund_rate_pct - 2.38).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// StartEvaluationRequest / SubmitEvaluationRequest
// ---------------------------------------------------------------------------

#[test]
fn start_evaluation_request_roundtrip() {
    let req = StartEvaluationRequest {
        delivery_entry_id: "entry-5".to_string(),
        template_id: "tmpl-1".to_string(),
        overall_comment: Some("Routine evaluation".to_string()),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: StartEvaluationRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.delivery_entry_id, "entry-5");
    assert_eq!(de.overall_comment.as_deref(), Some("Routine evaluation"));
}

#[test]
fn submit_evaluation_request_with_answers() {
    let req = SubmitEvaluationRequest {
        answers: vec![
            SubmitAnswerRequest {
                question_id: "q-1".to_string(),
                answer_text: Some("Yes".to_string()),
                manual_score: None,
                partial_credit_fraction: None,
                comment: None,
            },
        ],
        overall_comment: None,
    };
    assert_eq!(req.answers.len(), 1);
    assert_eq!(req.answers[0].answer_text.as_deref(), Some("Yes"));
}

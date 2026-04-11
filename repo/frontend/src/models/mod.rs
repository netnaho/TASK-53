use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub username: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRow {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub username: String,
    pub email: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRow {
    pub id: String,
    pub code: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentRow {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRow {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub trace_id: String,
}

// ============================================================
// Catalog / Package / Plan / Delivery types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceItemRow {
    pub id: String,
    pub org_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub unit_type: String,
    pub default_rate: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageRow {
    pub id: String,
    pub org_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRuleRow {
    pub id: String,
    pub package_id: String,
    pub service_item_id: String,
    pub rule_type: String,
    pub rate: f64,
    pub min_increment: Option<f64>,
    pub tier_config: Option<serde_json::Value>,
    pub max_units_per_delivery: Option<f64>,
    pub max_units_per_period: Option<i32>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDetail {
    pub package: PackageRow,
    pub rules: Vec<PackageRuleRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPlanRow {
    pub id: String,
    pub org_id: String,
    pub department_id: Option<String>,
    pub project_id: Option<String>,
    pub client_name: String,
    pub status: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPackageRow {
    pub id: String,
    pub plan_id: String,
    pub package_id: String,
    pub effective_date: String,
    pub end_date: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryEntryRow {
    pub id: String,
    pub org_id: String,
    pub plan_id: String,
    pub plan_package_id: String,
    pub service_item_id: String,
    pub provider_id: String,
    pub delivery_date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub units: f64,
    pub mileage: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryListResponse {
    pub data: Vec<DeliveryEntryRow>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRow {
    pub id: String,
    pub timestamp: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub org_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub trace_id: Option<String>,
}

// ============================================================
// Billing / Payment / Reconciliation types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeRow {
    pub id: String,
    pub org_id: String,
    pub delivery_entry_id: String,
    pub plan_id: String,
    pub invoice_id: Option<String>,
    pub rule_type: String,
    pub computed_units: f64,
    pub rate_applied: f64,
    pub gross_amount: f64,
    pub adjustment_total: f64,
    pub net_amount: f64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeAdjustmentRow {
    pub id: String,
    pub charge_id: String,
    pub adjusted_by: String,
    pub amount: f64,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceRow {
    pub id: String,
    pub org_id: String,
    pub plan_id: String,
    pub invoice_number: String,
    pub billing_period_start: String,
    pub billing_period_end: String,
    pub subtotal: f64,
    pub total_adjustments: f64,
    pub total_amount: f64,
    pub status: String,
    pub generated_by: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItemRow {
    pub id: String,
    pub invoice_id: String,
    pub charge_id: String,
    pub description: String,
    pub delivery_date: String,
    pub units: f64,
    pub unit_rate: f64,
    pub gross_amount: f64,
    pub adjustment_amount: f64,
    pub net_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedPaymentRow {
    pub id: String,
    pub org_id: String,
    pub invoice_id: String,
    pub fund_transaction_id: String,
    pub idempotency_key: String,
    pub payment_method: String,
    pub amount: f64,
    pub reference_number: Option<String>,
    pub payment_date: String,
    pub recorded_by: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedRefundRow {
    pub id: String,
    pub invoice_id: String,
    pub reason_code_id: String,
    pub amount: f64,
    pub reason_notes: Option<String>,
    pub refund_method: String,
    pub refund_date: String,
    pub recorded_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundReasonCodeRow {
    pub id: String,
    pub code: String,
    pub label: String,
    pub description: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReconciliationRunRow {
    pub id: String,
    pub org_id: String,
    pub period_start: String,
    pub period_end: String,
    pub total_charges: f64,
    pub total_adjustments: f64,
    pub total_invoiced: f64,
    pub total_paid: f64,
    pub total_refunded: f64,
    pub net_collected: f64,
    pub pending_charge_count: i64,
    pub invoiced_charge_count: i64,
    pub paid_invoice_count: i64,
    pub outstanding_balance: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedInvoices {
    pub data: Vec<InvoiceRow>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedPayments {
    pub data: Vec<RecordedPaymentRow>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedReconciliation {
    pub data: Vec<ReconciliationRunRow>,
    pub total: i64,
}

// ============================================================
// Scoring / Evaluation / Review types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringTemplateRow {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub description: Option<String>,
    pub rounding_interval: f64,
    pub max_score: f64,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationQuestionRow {
    pub id: String,
    pub template_id: String,
    pub question_text: String,
    pub question_type: String,
    pub weight: f64,
    pub max_points: f64,
    pub correct_answer: Option<String>,
    pub sort_order: i32,
    pub is_required: bool,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDetail {
    pub template: ScoringTemplateRow,
    pub questions: Vec<EvaluationQuestionRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationRow {
    pub id: String,
    pub org_id: String,
    pub delivery_entry_id: String,
    pub template_id: String,
    pub evaluator_id: String,
    pub status: String,
    pub prior_final_score: Option<f64>,
    pub raw_score: Option<f64>,
    pub weighted_score: Option<f64>,
    pub final_score: Option<f64>,
    pub requires_second_review: bool,
    pub score_delta: Option<f64>,
    pub second_reviewer_id: Option<String>,
    pub second_reviewed_at: Option<String>,
    pub overall_comment: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationAnswerRow {
    pub id: String,
    pub evaluation_id: String,
    pub question_id: String,
    pub answer_text: Option<String>,
    pub auto_score: f64,
    pub manual_score: f64,
    pub partial_credit_fraction: f64,
    pub final_score: f64,
    pub comment: Option<String>,
    pub graded_by: Option<String>,
    pub graded_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreReviewRow {
    pub id: String,
    pub evaluation_id: String,
    pub reviewer_id: String,
    pub score_before_review: f64,
    pub score_delta: f64,
    pub review_status: String,
    pub revised_score: Option<f64>,
    pub review_comment: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationDetail {
    pub evaluation: EvaluationRow,
    pub answers: Vec<EvaluationAnswerRow>,
    pub pending_review: Option<ScoreReviewRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedEvaluations {
    pub data: Vec<EvaluationRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedReviews {
    pub data: Vec<ScoreReviewRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// Request types for scoring

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartEvaluationRequest {
    pub delivery_entry_id: String,
    pub template_id: String,
    pub overall_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitAnswerRequest {
    pub question_id: String,
    pub answer_text: Option<String>,
    pub manual_score: Option<f64>,
    pub partial_credit_fraction: Option<f64>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitEvaluationRequest {
    pub answers: Vec<SubmitAnswerRequest>,
    pub overall_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondReviewRequest {
    pub action: String,
    pub revised_score: Option<f64>,
    pub review_comment: Option<String>,
}

// ============================================================
// Report / KPI / Export types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderVolumeRow {
    pub period: String,
    pub delivery_count: i64,
    pub unique_plans: i64,
    pub unique_providers: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueReportRow {
    pub period: String,
    pub gross_charges: f64,
    pub net_charges: f64,
    pub total_invoiced: f64,
    pub total_paid: f64,
    pub total_refunded: f64,
    pub refund_rate_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilizationRow {
    pub provider_id: String,
    pub period: String,
    pub total_visits: i64,
    pub total_units: f64,
    pub total_mileage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiSummary {
    pub period_start: String,
    pub period_end: String,
    pub attendance_rate_pct: f64,
    pub repurchase_rate_pct: f64,
    pub staff_utilization_pct: f64,
    pub avg_score: Option<f64>,
    pub second_review_rate_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub rows: Vec<serde_json::Value>,
    pub row_count: usize,
    pub masked: bool,
    pub export_log_id: String,
}

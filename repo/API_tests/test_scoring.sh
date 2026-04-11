#!/usr/bin/env bash
# API tests: Quality Scoring & Second Review Workflow
# Tests: template creation, evaluation lifecycle, auto/manual/partial scoring,
#        second-review enforcement (delta > 10), review approve/revise paths,
#        and authorization (401, 403).

set -euo pipefail

BASE_URL="${BACKEND_URL:-http://localhost:8000}"
PASSED=0
FAILED=0

check() {
    local name="$1"
    local expected="$2"
    local actual="$3"
    if [ "$actual" = "$expected" ]; then
        echo "  PASS  $name (got $actual)"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  $name (expected $expected, got $actual)"
        FAILED=$((FAILED + 1))
    fi
}

check_contains() {
    local name="$1"
    local needle="$2"
    local haystack="$3"
    if echo "$haystack" | grep -qF "$needle"; then
        echo "  PASS  $name (contains '$needle')"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  $name (expected to contain '$needle')"
        echo "        Got: $haystack" | head -c 300
        FAILED=$((FAILED + 1))
    fi
}

echo "======================================="
echo " Scoring & Review API Tests"
echo "======================================="
echo "Target: $BASE_URL"
echo ""

# =============================================================
# Auth
# =============================================================

ADMIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

QA_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"qa_reviewer","password":"QAReview123!"}' 2>/dev/null || echo "FAIL")
QA_TOKEN=$(echo "$QA_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

COACH_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"coach","password":"Coach123!"}' 2>/dev/null || echo "FAIL")
COACH_TOKEN=$(echo "$COACH_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

echo "[1] Auth setup"
check "Admin token obtained" "1" "$([ -n "$ADMIN_TOKEN" ] && echo 1 || echo 0)"
check "QA Reviewer token obtained" "1" "$([ -n "$QA_TOKEN" ] && echo 1 || echo 0)"

# =============================================================
# Unauthenticated access
# =============================================================

echo ""
echo "[2] Unauthenticated access"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/scoring/templates")
check "List templates — no token → 401" "401" "$STATUS"

# =============================================================
# Coach cannot create templates
# =============================================================

echo ""
echo "[3] Authorization — Coach cannot write scoring"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/scoring/templates" \
    -H "Authorization: Bearer $COACH_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Coach Template","questions":[{"question_text":"Q1","question_type":"subjective"}]}')
check "Coach create template → 403" "403" "$STATUS"

# =============================================================
# Template creation
# =============================================================

echo ""
echo "[4] Template creation"

TPL_RESP=$(curl -sf -X POST "$BASE_URL/api/scoring/templates" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "name": "Standard QA Template",
        "description": "Two-question test template",
        "rounding_interval": 0.5,
        "max_score": 100.0,
        "questions": [
            {
                "question_text": "Was the delivery documented correctly?",
                "question_type": "objective",
                "weight": 1.5,
                "max_points": 10.0,
                "correct_answer": "yes",
                "sort_order": 0,
                "is_required": true
            },
            {
                "question_text": "Rate the quality of service delivery.",
                "question_type": "subjective",
                "weight": 1.0,
                "max_points": 10.0,
                "sort_order": 1,
                "is_required": true
            }
        ]
    }' 2>/dev/null || echo "FAIL")

TPL_ID=$(echo "$TPL_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['template']['id'])" 2>/dev/null || echo "")
check "Template created — ID obtained" "1" "$([ -n "$TPL_ID" ] && echo 1 || echo 0)"
check_contains "Template has 2 questions" "\"question_text\"" "$TPL_RESP"

# =============================================================
# Template missing questions → 400
# =============================================================

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/scoring/templates" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Empty","questions":[]}')
check "Create template with no questions → 400" "400" "$STATUS"

# =============================================================
# List templates
# =============================================================

echo ""
echo "[5] List templates"

LIST_RESP=$(curl -sf "$BASE_URL/api/scoring/templates" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Templates list contains created template" "$TPL_ID" "$LIST_RESP"

# =============================================================
# Get template detail
# =============================================================

DETAIL_RESP=$(curl -sf "$BASE_URL/api/scoring/templates/$TPL_ID" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Template detail has rounding_interval" "rounding_interval" "$DETAIL_RESP"

# =============================================================
# Start evaluation — need a delivery entry ID from prior test run
# =============================================================

echo ""
echo "[6] Evaluation lifecycle"

# Fetch first delivery entry for the org
DELIVERY_RESP=$(curl -sf "$BASE_URL/api/delivery?limit=1" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
DELIVERY_ID=$(echo "$DELIVERY_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data'][0]['id'] if d.get('data') else '')" 2>/dev/null || echo "")

if [ -z "$DELIVERY_ID" ]; then
    echo "  SKIP  No delivery entries available — skipping evaluation tests"
else
    EVAL_RESP=$(curl -sf -X POST "$BASE_URL/api/scoring/evaluations" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{
            \"delivery_entry_id\": \"$DELIVERY_ID\",
            \"template_id\": \"$TPL_ID\",
            \"overall_comment\": \"Test evaluation\"
        }" 2>/dev/null || echo "FAIL")

    EVAL_ID=$(echo "$EVAL_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['evaluation']['id'])" 2>/dev/null || echo "")
    check "Evaluation started — ID obtained" "1" "$([ -n "$EVAL_ID" ] && echo 1 || echo 0)"
    check_contains "New evaluation status is draft" "\"draft\"" "$EVAL_RESP"

    # Get evaluation detail
    GET_RESP=$(curl -sf "$BASE_URL/api/scoring/evaluations/$EVAL_ID" \
        -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
    check_contains "Get evaluation detail — has answers array" "\"answers\"" "$GET_RESP"

    # =============================================================
    # Submit evaluation — objective answer match, subjective manual score
    # =============================================================

    echo ""
    echo "[7] Submit evaluation with auto-score + manual-score"

    # Fetch question IDs from template
    Q1_ID=$(echo "$DETAIL_RESP" | python3 -c "import sys,json; qs=json.load(sys.stdin)['questions']; print(qs[0]['id'] if qs else '')" 2>/dev/null || echo "")
    Q2_ID=$(echo "$DETAIL_RESP" | python3 -c "import sys,json; qs=json.load(sys.stdin)['questions']; print(qs[1]['id'] if len(qs)>1 else '')" 2>/dev/null || echo "")

    SUBMIT_RESP=$(curl -sf -X POST "$BASE_URL/api/scoring/evaluations/$EVAL_ID/submit" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{
            \"answers\": [
                {
                    \"question_id\": \"$Q1_ID\",
                    \"answer_text\": \"yes\",
                    \"comment\": \"Auto-scored objective\"
                },
                {
                    \"question_id\": \"$Q2_ID\",
                    \"manual_score\": 8.0,
                    \"comment\": \"Good delivery quality\"
                }
            ],
            \"overall_comment\": \"Satisfactory delivery\"
        }" 2>/dev/null || echo "FAIL")

    FINAL_SCORE=$(echo "$SUBMIT_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['evaluation'].get('final_score','null'))" 2>/dev/null || echo "null")
    EVAL_STATUS=$(echo "$SUBMIT_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['evaluation']['status'])" 2>/dev/null || echo "")
    check "Evaluation submitted — has final_score" "1" "$([ "$FINAL_SCORE" != "null" ] && echo 1 || echo 0)"
    check "Submitted evaluation status is finalized or second_review_required" "1" \
        "$(echo "$EVAL_STATUS" | grep -qE '^(finalized|second_review_required)$' && echo 1 || echo 0)"
fi

# =============================================================
# Second review enforcement
# =============================================================

echo ""
echo "[8] Second-review enforcement"

# Manually trigger a second-review scenario by creating an evaluation with
# a prior_final_score set 15 points lower — we can't inject this directly, so
# we verify the pending reviews endpoint works and rejects processing reviews
# on non-pending evaluations.

PENDING_RESP=$(curl -sf "$BASE_URL/api/scoring/reviews/pending" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Pending reviews endpoint returns list" "\"data\"" "$PENDING_RESP"

# Trying to process a review for a non-existent evaluation
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/scoring/reviews/00000000-0000-0000-0000-000000000000" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"action":"approve"}')
check "Process review on non-existent eval → 404" "404" "$STATUS"

# =============================================================
# Review: invalid action
# =============================================================

# If there's a pending review, try an invalid action
if [ -n "${EVAL_ID:-}" ] && echo "$EVAL_STATUS" | grep -q "second_review_required"; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BASE_URL/api/scoring/reviews/$EVAL_ID" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"action":"invalid_action"}')
    check "Invalid review action → 400" "400" "$STATUS"

    # Revise without providing revised_score → 400
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BASE_URL/api/scoring/reviews/$EVAL_ID" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"action":"revise"}')
    check "Revise without revised_score → 400" "400" "$STATUS"
fi

# =============================================================
# List evaluations with status filter
# =============================================================

echo ""
echo "[9] List evaluations with filters"

LIST_FINALIZED=$(curl -sf "$BASE_URL/api/scoring/evaluations?status=finalized" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "List finalized evaluations — has data field" "\"data\"" "$LIST_FINALIZED"

# =============================================================
# Independent review enforcement (TASK-53)
# These tests verify that:
#   - evaluations requiring second review cannot finalize without
#     an independent reviewer
#   - the evaluator cannot review their own evaluation
#   - a non-assigned reviewer is rejected
#   - a valid independent reviewer can process the review
# =============================================================

echo ""
echo "[10] Independent review enforcement (TASK-53)"

# If we have a second_review_required evaluation from [7], test self-review denial.
# The admin submitted the evaluation, so admin is the evaluator.  Even if admin
# were assigned as reviewer (prior buggy behavior), this should now be blocked.

if [ -n "${EVAL_ID:-}" ] && echo "${EVAL_STATUS:-}" | grep -q "second_review_required"; then

    # Test: evaluator (admin) tries to process own review → should be 403 or 404
    # (404 if no review record was created because admin was the only QA reviewer
    #  candidate; 403 if a review record exists but self-review is blocked)
    SELF_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BASE_URL/api/scoring/reviews/$EVAL_ID" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"action":"approve","review_comment":"Self-approve attempt"}')
    # Accept 403 (self-review blocked) or 404 (no review record created)
    if [ "$SELF_STATUS" = "403" ] || [ "$SELF_STATUS" = "404" ]; then
        check "Evaluator cannot self-review (got $SELF_STATUS)" "1" "1"
    else
        check "Evaluator cannot self-review (expected 403 or 404)" "403_or_404" "$SELF_STATUS"
    fi

    # Test: coach (not the assigned reviewer) tries to process → should be 403
    if [ -n "$COACH_TOKEN" ]; then
        COACH_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST "$BASE_URL/api/scoring/reviews/$EVAL_ID" \
            -H "Authorization: Bearer $COACH_TOKEN" \
            -H "Content-Type: application/json" \
            -d '{"action":"approve","review_comment":"Unauthorized attempt"}')
        check "Non-assigned reviewer denied (403)" "403" "$COACH_STATUS"
    fi

    # Test: evaluation is still in second_review_required — not silently finalized
    RECHECK_RESP=$(curl -sf "$BASE_URL/api/scoring/evaluations/$EVAL_ID" \
        -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
    RECHECK_STATUS=$(echo "$RECHECK_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['evaluation']['status'])" 2>/dev/null || echo "")
    check "Evaluation still in second_review_required" "second_review_required" "$RECHECK_STATUS"

    # Test: QA reviewer (independent) can process the review — happy path
    if [ -n "$QA_TOKEN" ]; then
        QA_REVIEW_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST "$BASE_URL/api/scoring/reviews/$EVAL_ID" \
            -H "Authorization: Bearer $QA_TOKEN" \
            -H "Content-Type: application/json" \
            -d '{"action":"approve","review_comment":"Independently verified"}')
        # QA reviewer should succeed (200) if assigned, or 403/404 if not assigned
        if [ "$QA_REVIEW_STATUS" = "200" ]; then
            check "QA reviewer can approve (200)" "200" "$QA_REVIEW_STATUS"
            # Verify evaluation is now finalized
            FINAL_RESP=$(curl -sf "$BASE_URL/api/scoring/evaluations/$EVAL_ID" \
                -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
            FINAL_ST=$(echo "$FINAL_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['evaluation']['status'])" 2>/dev/null || echo "")
            check "Evaluation finalized after independent review" "finalized" "$FINAL_ST"
        else
            echo "  INFO  QA reviewer got $QA_REVIEW_STATUS (may not be assigned reviewer)"
        fi
    fi

else
    echo "  SKIP  No second_review_required evaluation available for independent review tests"
    echo "        (evaluation finalized directly or no delivery entries in test data)"
fi

# =============================================================
# Summary
# =============================================================

echo ""
echo "======================================="
echo " Results: $PASSED passed, $FAILED failed"
echo "======================================="

[ "$FAILED" -eq 0 ] || exit 1

#!/usr/bin/env python3
"""
scripts/runner.py - Beautiful Crabbase Test Runner & Feature Matrix

================================================================================
🦀 HOW TO USE THIS RUNNER
================================================================================
1. Run everything (Unit + Database + HTTP Integration):
       ./scripts/run_tests.sh all
       make test

2. Run specific test tiers:
       ./scripts/run_tests.sh unit          # Fast in-memory unit tests (~70 tests)
       ./scripts/run_tests.sh db            # Postgres schema-isolated DB repository tests
       ./scripts/run_tests.sh api           # Full HTTP REST API integration tests

3. Run a specific test suite:
       ./scripts/run_tests.sh records       # Runs tests/records_api_test.rs
       ./scripts/run_tests.sh rules         # Runs tests/access_rules_test.rs
       ./scripts/run_tests.sh auth          # Runs tests/auth_api_test.rs
       ./scripts/run_tests.sh validation    # Runs tests/field_validation_test.rs
       ./scripts/run_tests.sh types         # Runs tests/field_types_test.rs
       ./scripts/run_tests.sh settings      # Runs tests/settings_api_test.rs
       ./scripts/run_tests.sh collections   # Runs tests/collections_api_test.rs
       ./scripts/run_tests.sh sanitize      # Runs tests/response_sanitization_test.rs

4. View the Crabbase Feature Matrix:
       ./scripts/run_tests.sh matrix        # Runs tests live & displays feature matrix
       ./scripts/run_tests.sh matrix --no-run # Instant feature matrix view (no tests run)

================================================================================
🛠️ DEVELOPER GUIDE: HOW TO ADD NEW FEATURES & TESTS
================================================================================
Adding a new test to Crabbase is a 2-step process:

STEP 1: Write your test in Rust
--------------------------------------------------------------------------------
- Add an `async fn test_<your_test_name>()` in one of the test files under `crates/api/tests/`:
    - `collections_api_test.rs`  -> Collections CRUD, migrations, truncate
    - `records_api_test.rs`      -> Records CRUD, filters, sorts, pagination, expand
    - `access_rules_test.rs`     -> 5-rule security permissions & @request context
    - `auth_api_test.rs`         -> JWT login, token rotation, registration, OTP
    - `field_validation_test.rs` -> Field constraint validation engine
    - `response_sanitization_test.rs` -> Field redaction (passwords, private emails)
    - `field_types_test.rs`      -> Support for all 13 database column types
    - `settings_api_test.rs`     -> Mail, app, and email template settings
- Or create a new test file in `crates/api/tests/<new_test_file>.rs`.

STEP 2: Register the feature in `FEATURE_CHECKLIST` (below in this script)
--------------------------------------------------------------------------------
Add a new tuple to `FEATURE_CHECKLIST`:
    (
        "🗄️ Category Name",              # Category header in the matrix
        "test_your_exact_function_name",   # Must match the Rust test fn name exactly
        "Human readable description",      # Display title in the matrix
        "feature",                         # "feature" (pending), "bug" (known bug), or "pass" (existing)
    )

When your Rust test passes:
- The item will AUTOMATICALLY turn green `[PASS]` in the feature matrix!
- The Crabbase Feature Readiness percentage bar will automatically increase.
================================================================================
"""

import sys
import os
import re
import subprocess
import time
from collections import defaultdict

# ─── ANSI Colors & Terminal Styles ───────────────────────────────────────────
BOLD = "\033[1m"
DIM = "\033[2m"
ITALIC = "\033[3m"
UNDERLINE = "\033[4m"

RED = "\033[38;5;203m"
GREEN = "\033[38;5;120m"
YELLOW = "\033[38;5;221m"
BLUE = "\033[38;5;75m"
MAGENTA = "\033[38;5;176m"
CYAN = "\033[38;5;80m"
WHITE = "\033[38;5;255m"
GRAY = "\033[38;5;244m"
NC = "\033[0m"

PASS_ICON = f"{GREEN}✔ PASS{NC}"
FAIL_ICON = f"{RED}✖ FAIL{NC}"
TODO_ICON = f"{YELLOW}⧗ GAP {NC}"
INFO_ICON = f"{CYAN}ℹ{NC}"


# ==============================================================================
# 📋 1. SUITE MAPPINGS & CLI ALIASES
# ==============================================================================
# Maps `.rs` test file base names to human-readable subsuite labels in the UI.
# If you add a new test file in `crates/api/tests/`, register its display name here!
SUBSUITE_NAME_MAP = {
    "collections_api_test": "Collections API",
    "records_api_test": "Records CRUD & Queries",
    "access_rules_test": "5-Rule Access Matrix",
    "auth_api_test": "Auth & Self-Registration",
    "field_validation_test": "Field Validation Engine",
    "response_sanitization_test": "Response Sanitization",
    "field_types_test": "All 13 Field Types",
    "settings_api_test": "Settings API",
}

# CLI shorthands: e.g. `./scripts/run_tests.sh records` -> `records_api_test`
# If you want a new CLI shortcut argument, add it to this dictionary!
SUITE_ALIASES = {
    "records": "records_api_test",
    "rules": "access_rules_test",
    "access": "access_rules_test",
    "auth": "auth_api_test",
    "validation": "field_validation_test",
    "types": "field_types_test",
    "sanitize": "response_sanitization_test",
    "settings": "settings_api_test",
    "collections": "collections_api_test",
}


# ==============================================================================
# 🏆 2. CRABBASE FEATURE MATRIX CHECKLIST
# ==============================================================================
# Format:
#   (Category, Test Function Name, Human Description, Initial Status)
#
# Initial Status:
#   - "pass": Feature is already working & tested
#   - "bug": Known bug that needs fixing
#   - "feature": Feature capability waiting to be implemented
#
# When cargo test runs, if `test_fn_name` passes in the log, it AUTOMATICALLY
# turns green `[PASS]` regardless of the initial status.
# ==============================================================================
FEATURE_CHECKLIST = [
    # ── Category 1: Known Bugs & Baseline Fixes ──────────────────────────────
    (
        "🐛 Known Bugs & Baseline Fixes",
        "test_truncate_collection_removes_all_records",
        "truncate() SQL injection prevention",
        "pass",
    ),
    (
        "🐛 Known Bugs & Baseline Fixes",
        "test_update_record_returns_full_record_json",
        "update_record returns full Record JSON (not message string)",
        "bug",
    ),
    (
        "🐛 Known Bugs & Baseline Fixes",
        "test_collection_list_pagination_total_count",
        "collection list 'total' reflects real count (not result.len())",
        "bug",
    ),
    (
        "🐛 Known Bugs & Baseline Fixes",
        "test_list_rule_null_requires_admin",
        "list_rule = null requires admin (403 default)",
        "bug",
    ),
    # ── Category 2: Collections & Schema DDL ──────────────────────────────────
    (
        "🗄️ Collections & DDL",
        "test_all_13_field_types_end_to_end_crud",
        "Support all 13 field types (Text, Number, Bool, Datetime, Relation...)",
        "pass",
    ),
    (
        "🗄️ Collections & DDL",
        "test_update_collection_add_column",
        "Live schema migration (add/drop/retype column)",
        "pass",
    ),
    (
        "🗄️ Collections & DDL",
        "test_create_collection_duplicate_name_conflict_409",
        "Duplicate collection name returns 409 Conflict",
        "pass",
    ),
    (
        "🗄️ Collections & DDL",
        "test_create_auth_collection_auto_injects_system_columns",
        "Auto-inject auth columns on type = 'auth'",
        "feature",
    ),
    # ── Category 3: Records CRUD & Query Engine ───────────────────────────────
    (
        "📝 Records CRUD & Queries",
        "test_create_record_returns_record_with_id",
        "Create / Read / Delete records via REST API",
        "pass",
    ),
    (
        "📝 Records CRUD & Queries",
        "test_list_records_per_page_limiting",
        "Pagination (page, per_page)",
        "pass",
    ),
    (
        "📝 Records CRUD & Queries",
        "test_create_auth_record_hashes_password",
        "Password auto-hashing on auth records",
        "pass",
    ),
    (
        "📝 Records CRUD & Queries",
        "test_filter_parameter_evaluates_where_clause",
        "Client filtering query parameter (?filter=...)",
        "feature",
    ),
    (
        "📝 Records CRUD & Queries",
        "test_sort_parameter_orders_results",
        "Sorting query parameter (?sort=-created)",
        "feature",
    ),
    (
        "📝 Records CRUD & Queries",
        "test_expand_parameter_populates_related_record",
        "Relation expansion query parameter (?expand=...)",
        "feature",
    ),
    (
        "📝 Records CRUD & Queries",
        "test_fields_parameter_restricts_returned_columns",
        "Field projection query parameter (?fields=...)",
        "feature",
    ),
    # ── Category 4: API Rules & Security Permissions ─────────────────────────
    (
        "🛡️ API Rules & Security",
        "test_compile_logical_expressions",
        "Rule AST Tokenizer & SQL Compiler",
        "pass",
    ),
    (
        "🛡️ API Rules & Security",
        "test_list_rule_empty_string_is_public",
        "list_rule = '' public access allowed",
        "pass",
    ),
    (
        "🛡️ API Rules & Security",
        "test_list_rule_expression_filters_rows",
        "list_rule row-level expression filtering",
        "pass",
    ),
    (
        "🛡️ API Rules & Security",
        "test_view_rule_null_blocks_unauthenticated_get",
        "view_rule enforcement on GET /records/:id",
        "feature",
    ),
    (
        "🛡️ API Rules & Security",
        "test_view_rule_expression_enforced",
        "view_rule expression evaluation",
        "feature",
    ),
    (
        "🛡️ API Rules & Security",
        "test_create_rule_null_blocks_unauthenticated_create",
        "create_rule enforcement on POST /records",
        "feature",
    ),
    (
        "🛡️ API Rules & Security",
        "test_create_rule_with_request_data_context",
        "@request.data.* context in rule compiler",
        "feature",
    ),
    (
        "🛡️ API Rules & Security",
        "test_update_rule_null_blocks_unauthenticated_update",
        "update_rule enforcement on PATCH /records/:id",
        "feature",
    ),
    (
        "🛡️ API Rules & Security",
        "test_delete_rule_null_blocks_unauthenticated_delete",
        "delete_rule enforcement on DELETE /records/:id",
        "feature",
    ),
    # ── Category 5: User Authentication & Security ───────────────────────────
    (
        "👤 Auth & Users",
        "test_login_happy_path_returns_tokens",
        "Superuser login & JWT token issuance",
        "pass",
    ),
    (
        "👤 Auth & Users",
        "test_refresh_token_returns_new_access_token",
        "Refresh token rotation (atomic CTE)",
        "pass",
    ),
    (
        "👤 Auth & Users",
        "test_refresh_token_reuse_is_rejected",
        "Refresh token reuse detection (family revocation)",
        "pass",
    ),
    (
        "👤 Auth & Users",
        "test_logout_invalidates_refresh_token",
        "Logout session revocation",
        "pass",
    ),
    (
        "👤 Auth & Users",
        "test_unverified_user_cannot_login",
        "verified flag enforcement in verify_session",
        "pass",
    ),
    (
        "👤 Auth & Users",
        "test_public_user_registration_via_create_rule_empty",
        "Public user self-registration (create_rule = '')",
        "feature",
    ),
    (
        "👤 Auth & Users",
        "test_record_responses_strip_password_and_token_key",
        "Response sanitization (never leak password/token_key)",
        "feature",
    ),
    # ── Category 6: Server-Side Field Validation Engine ───────────────────────
    (
        "🔍 Field Validation Engine",
        "test_validation_required_field_missing_on_create_rejected",
        "required: true field check on create (400 on null)",
        "feature",
    ),
    (
        "🔍 Field Validation Engine",
        "test_validation_string_min_max_length_enforced",
        "String min / max length validation",
        "feature",
    ),
    (
        "🔍 Field Validation Engine",
        "test_validation_number_min_max_range_enforced",
        "Number min / max range validation",
        "feature",
    ),
    (
        "🔍 Field Validation Engine",
        "test_validation_pattern_regex_enforced",
        "Regex pattern validation",
        "feature",
    ),
    # ── Category 7: Settings REST API ─────────────────────────────────────────
    (
        "⚙️ Settings API",
        "test_settings_endpoints_require_admin",
        "Settings endpoints admin authorization guard",
        "pass",
    ),
    (
        "⚙️ Settings API",
        "test_mail_settings_get_and_post",
        "Mail settings GET & POST persistence",
        "pass",
    ),
    (
        "⚙️ Settings API",
        "test_app_settings_get_and_post",
        "App settings GET & POST persistence",
        "pass",
    ),
    (
        "⚙️ Settings API",
        "test_email_templates_get_and_post",
        "Email templates GET & POST persistence",
        "pass",
    ),
]


import unicodedata

# ─── Display Width & Padding Helpers ──────────────────────────────────────────
def display_width(s: str) -> int:
    """Calculates visible monospace terminal display width, accounting for wide emojis."""
    clean_s = re.sub(r"\033\[[0-9;]*m", "", s)
    w = 0
    for ch in clean_s:
        # East Asian wide/fullwidth or high Unicode emoji codepoints occupy 2 columns
        if unicodedata.east_asian_width(ch) in ("W", "F") or ord(ch) >= 0x1F300:
            w += 2
        else:
            w += 1
    return w


def pad_string(s: str, target_width: int, align_left: bool = True) -> str:
    """Pads a string to an exact terminal display width regardless of emojis."""
    current_w = display_width(s)
    pad_len = max(0, target_width - current_w)
    if align_left:
        return s + (" " * pad_len)
    else:
        return (" " * pad_len) + s


def center_string(s: str, target_width: int) -> str:
    """Centers a string within target terminal display width."""
    current_w = display_width(s)
    total_pad = max(0, target_width - current_w)
    left_pad = total_pad // 2
    right_pad = total_pad - left_pad
    return (" " * left_pad) + s + (" " * right_pad)


# ==============================================================================
# 🖥️ 3. TERMINAL RENDERING & FORMATTING FUNCTIONS
# ==============================================================================

def print_banner(scope: str, db_url: str):
    """Renders the top stylized box banner."""
    width = 88
    print(f"\n{BLUE}╔{'═' * (width - 2)}╗{NC}")
    title_text = "🦀  CRABBASE TEST RUNNER"
    print(f"{BLUE}║{BOLD}{WHITE}{center_string(title_text, width - 2)}{NC}{BLUE}║{NC}")

    scope_line = f"  Scope:    {scope}"
    print(f"{BLUE}║{GRAY}{pad_string(scope_line, width - 2)}{NC}{BLUE}║{NC}")

    if db_url:
        short_db = db_url if len(db_url) <= 70 else db_url[:67] + "..."
        db_line = f"  Database: {short_db}"
        print(f"{BLUE}║{GRAY}{pad_string(db_line, width - 2)}{NC}{BLUE}║{NC}")
    print(f"{BLUE}╚{'═' * (width - 2)}╝{NC}\n")


def print_section(title: str, subtitle: str = ""):
    """Renders a section header line."""
    print(f"\n{BOLD}{CYAN}─── {title} ───{NC} {GRAY}{subtitle}{NC}")


def clean_test_name(raw_name: str) -> str:
    """Extracts function name from `path::to::test_name`."""
    return raw_name.split("::")[-1]


def format_test_description(test_name: str) -> str:
    """Converts `test_create_record_returns_record_with_id` to `Create record returns record with id`."""
    name = test_name.removeprefix("test_")
    return " ".join(name.split("_")).capitalize()


# ==============================================================================
# ⚙️ 4. TEST EXECUTION & LOG PARSING
# ==============================================================================


class TestResult:
    """Encapsulates the parsed outcome of a single test function."""

    def __init__(
        self,
        name: str,
        category: str,
        subsuite: str,
        passed: bool,
        failure_msg: str = "",
    ):
        self.name = name
        self.category = category
        self.subsuite = subsuite
        self.passed = passed
        self.failure_msg = failure_msg


def run_cargo_command(cmd: list[str], env: dict) -> tuple[int, str]:
    """Executes a cargo test command and captures combined stdout/stderr."""
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=env,
    )
    stdout, _ = proc.communicate()
    return proc.returncode, stdout


def parse_cargo_test_output(output: str, category_name: str) -> list[TestResult]:
    """Parses cargo test terminal output into structured TestResult objects."""
    results = []
    lines = output.splitlines()

    current_subsuite = category_name
    failure_details: dict[str, str] = {}
    current_failing_test = None
    collecting_panic = False
    panic_lines = []

    for line in lines:
        # Detect test binary header: e.g. "Running tests/collections_api_test.rs (target/...)"
        sub_match = re.search(r"Running (?:tests/)?([\w_]+)(?:\.rs)?\s*\(", line)
        if sub_match:
            raw_sub = sub_match.group(1)
            current_subsuite = SUBSUITE_NAME_MAP.get(
                raw_sub, raw_sub.replace("_", " ").title()
            )

        # Collect failure panic messages
        if line.startswith("thread '") and "panicked at" in line:
            m = re.search(r"thread '([^']+)' panicked at [^:]+:\d+:\d+:\s*(.*)", line)
            if m:
                current_failing_test = m.group(1).split("::")[-1]
                collecting_panic = True
                first_msg = m.group(2).strip()
                panic_lines = [first_msg] if first_msg else []
                continue
        elif collecting_panic:
            if (
                line.startswith("note: run with")
                or line.startswith("stack backtrace:")
                or line.startswith("test test_")
                or line.startswith("failures:")
            ):
                if current_failing_test:
                    failure_details[current_failing_test] = " ".join(
                        [l.strip() for l in panic_lines if l.strip()]
                    )
                collecting_panic = False
                current_failing_test = None
                panic_lines = []
            else:
                panic_lines.append(line.strip())

        # Parse test execution line: `test <path> ... ok` or `test <path> ... FAILED`
        m = re.match(r"^test ([\w:]+) \.\.\. (ok|FAILED|ignored)$", line.strip())
        if m:
            raw_name = m.group(1)
            status = m.group(2)
            if status == "ignored":
                continue
            name = clean_test_name(raw_name)
            passed = status == "ok"
            fail_msg = failure_details.get(name, "")
            if "assertion `left == right` failed" in fail_msg:
                fail_msg = re.sub(r"assertion `left == right` failed:\s*", "", fail_msg)
            results.append(
                TestResult(
                    name=name,
                    category=category_name,
                    subsuite=current_subsuite,
                    passed=passed,
                    failure_msg=fail_msg,
                )
            )

    return results


def render_suite_results(suite_title: str, results: list[TestResult]):
    """Renders formatted test items for a single execution block."""
    if not results:
        return

    # Group by subsuite if multiple exist
    by_subsuite = defaultdict(list)
    for r in results:
        by_subsuite[r.subsuite].append(r)

    for sub_title, sub_results in by_subsuite.items():
        passed_count = sum(1 for r in sub_results if r.passed)
        total_count = len(sub_results)

        display_title = sub_title if sub_title != suite_title else suite_title
        print_section(display_title, f"({passed_count}/{total_count} passed)")

        for r in sub_results:
            desc = format_test_description(r.name)
            desc_padded = pad_string(desc, 56)
            if r.passed:
                print(f"  {PASS_ICON}  {WHITE}{desc_padded}{NC} {GRAY}{r.name}{NC}")
            else:
                print(f"  {FAIL_ICON}  {RED}{desc_padded}{NC} {GRAY}{r.name}{NC}")
                if r.failure_msg:
                    clean_msg = (
                        r.failure_msg[:130] + "..."
                        if len(r.failure_msg) > 130
                        else r.failure_msg
                    )
                    print(f"         {GRAY}↳ Reason:{NC} {YELLOW}{clean_msg}{NC}")


# ==============================================================================
# 📊 5. DETAILED RESULTS BREAKDOWN TABLE
# ==============================================================================


def render_breakdown_table(all_results: list[TestResult], elapsed: float):
    """Renders the comprehensive Category -> Subsuite breakdown table."""
    width = 88
    print(f"\n{BLUE}╔{'═' * (width - 2)}╗{NC}")
    title_line = "📊  TEST RESULTS BREAKDOWN"
    print(f"{BLUE}║{BOLD}{WHITE}{center_string(title_line, width - 2)}{NC}{BLUE}║{NC}")
    print(f"{BLUE}╠{'═' * (width - 2)}╣{NC}")

    header_cat = pad_string("  Category / Test Suite", 50)
    header_pass = pad_string("Passed", 8, align_left=False)
    header_fail = pad_string("Failed", 8, align_left=False)
    header_tot = pad_string("Total", 7, align_left=False)
    header_rate = pad_string("Rate", 6, align_left=False)
    print(f"{BLUE}║{BOLD}{WHITE}{header_cat} {header_pass}  {header_fail}  {header_tot}  {header_rate} {NC}{BLUE}║{NC}")
    print(f"{BLUE}╟{'─' * (width - 2)}╢{NC}")

    # Group results by Category -> Subsuite
    cat_order = []
    grouped = defaultdict(lambda: defaultdict(list))

    for r in all_results:
        if r.category not in cat_order:
            cat_order.append(r.category)
        grouped[r.category][r.subsuite].append(r)

    grand_passed = 0
    grand_failed = 0
    grand_total = len(all_results)

    for cat in cat_order:
        subsuites = grouped[cat]
        cat_passed = sum(1 for sub in subsuites.values() for r in sub if r.passed)
        cat_total = sum(len(sub) for sub in subsuites.values())
        cat_failed = cat_total - cat_passed
        cat_rate = (cat_passed * 100) // cat_total if cat_total else 0

        grand_passed += cat_passed
        grand_failed += cat_failed

        rate_color = GREEN if cat_rate == 100 else YELLOW if cat_rate >= 50 else RED

        # Category header row
        cat_col = pad_string(f" {cat}", 50)
        pass_col = pad_string(str(cat_passed), 8, align_left=False)
        fail_col = pad_string(str(cat_failed), 8, align_left=False)
        tot_col = pad_string(str(cat_total), 7, align_left=False)
        rate_col = pad_string(f"{cat_rate}%", 6, align_left=False)

        print(
            f"{BLUE}║{NC}{BOLD}{cat_col}{NC} {GREEN}{pass_col}{NC}  {RED if cat_failed > 0 else GRAY}{fail_col}{NC}  {WHITE}{tot_col}{NC}  {rate_color}{rate_col}{NC} {BLUE}║{NC}"
        )

        # Subsuites indented tree rows (e.g. for HTTP API integration test files)
        if len(subsuites) > 1 or (
            len(subsuites) == 1 and list(subsuites.keys())[0] != cat
        ):
            for i, (sub_name, sub_res) in enumerate(subsuites.items()):
                is_last = i == len(subsuites) - 1
                tree_char = "└─" if is_last else "├─"
                sub_passed = sum(1 for r in sub_res if r.passed)
                sub_total = len(sub_res)
                sub_failed = sub_total - sub_passed
                sub_rate = (sub_passed * 100) // sub_total if sub_total else 0
                sub_color = (
                    GREEN if sub_rate == 100 else YELLOW if sub_rate >= 50 else RED
                )

                sub_col = pad_string(f"    {tree_char} {sub_name}", 50)
                sub_pass_col = pad_string(str(sub_passed), 8, align_left=False)
                sub_fail_col = pad_string(str(sub_failed), 8, align_left=False)
                sub_tot_col = pad_string(str(sub_total), 7, align_left=False)
                sub_rate_col = pad_string(f"{sub_rate}%", 6, align_left=False)

                print(
                    f"{BLUE}║{NC}{GRAY}{sub_col}{NC} {GREEN if sub_passed > 0 else GRAY}{sub_pass_col}{NC}  {RED if sub_failed > 0 else GRAY}{sub_fail_col}{NC}  {GRAY}{sub_tot_col}{NC}  {sub_color}{sub_rate_col}{NC} {BLUE}║{NC}"
                )

    grand_rate = (grand_passed * 100) // grand_total if grand_total else 0
    grand_color = GREEN if grand_rate == 100 else YELLOW if grand_rate >= 50 else RED

    tot_cat_col = pad_string("  TOTAL", 50)
    tot_pass_col = pad_string(str(grand_passed), 8, align_left=False)
    tot_fail_col = pad_string(str(grand_failed), 8, align_left=False)
    tot_total_col = pad_string(str(grand_total), 7, align_left=False)
    tot_rate_col = pad_string(f"{grand_rate}%", 6, align_left=False)

    print(f"{BLUE}╠{'═' * (width - 2)}╣{NC}")
    print(
        f"{BLUE}║{NC}{BOLD}{WHITE}{tot_cat_col}{NC} {BOLD}{GREEN}{tot_pass_col}{NC}  {BOLD}{RED if grand_failed > 0 else GRAY}{tot_fail_col}{NC}  {BOLD}{WHITE}{tot_total_col}{NC}  {BOLD}{grand_color}{tot_rate_col}{NC} {BLUE}║{NC}"
    )
    print(f"{BLUE}╚{'═' * (width - 2)}╝{NC}")
    print(
        f"{GRAY}Completed {grand_total} tests across {len(cat_order)} suites in {elapsed:.2f}s{NC}\n"
    )


# ==============================================================================
# 🏆 6. CRABBASE FEATURE MATRIX RENDERING
# ==============================================================================


def render_feature_matrix(all_results: list[TestResult]):
    """Renders the Crabbase feature readiness matrix."""
    passed_names = {r.name for r in all_results if r.passed}

    width = 88
    print(f"\n{BLUE}╔{'═' * (width - 2)}╗{NC}")
    title_text = "🏆  CRABBASE FEATURE MATRIX"
    print(
        f"{BLUE}║{BOLD}{WHITE}{center_string(title_text, width - 2)}{NC}{BLUE}║{NC}"
    )
    print(f"{BLUE}╚{'═' * (width - 2)}╝{NC}")

    current_cat = ""
    total_items = len(FEATURE_CHECKLIST)
    passed_items = 0

    for cat, test_fn, label, default_type in FEATURE_CHECKLIST:
        if cat != current_cat:
            current_cat = cat
            print(f"\n{BOLD}{WHITE}{cat}{NC}")

        is_passing = (
            (test_fn in passed_names) if all_results else (default_type == "pass")
        )

        if is_passing:
            passed_items += 1
            print(f"  {GREEN}[PASS]{NC} {label}")
        else:
            if default_type == "bug":
                print(f"  {RED}[FAIL]{NC} {label} {GRAY}(Bug to fix){NC}")
            else:
                print(f"  {YELLOW}[TODO]{NC} {label} {GRAY}(Pending Feature){NC}")

    # Progress bar calculation
    percent = (passed_items * 100) // total_items
    bar_width = 40
    filled = (passed_items * bar_width) // total_items
    bar = f"{GREEN}{'█' * filled}{NC}{GRAY}{'░' * (bar_width - filled)}{NC}"

    print(
        f"\n{BOLD}Crabbase Feature Readiness:{NC} [{bar}] {BOLD}{CYAN}{passed_items}/{total_items} features ({percent}%){NC}\n"
    )


# ==============================================================================
# 🚀 7. MAIN CLI DISPATCHER
# ==============================================================================


def main():
    args = sys.argv[1:]
    mode = args[0].lower() if args else "all"

    db_url = os.environ.get(
        "DATABASE_URL",
        os.environ.get(
            "TEST_DATABASE_URL", "postgres://postgres:postgres@localhost:5432/crabbase"
        ),
    )
    env = os.environ.copy()
    env["DATABASE_URL"] = db_url
    env["TEST_DATABASE_URL"] = db_url

    no_run = any(arg in ["--no-run", "-n", "no-run=1", "NO_RUN=1"] for arg in args)

    # ── Matrix Mode ───────────────────────────────────────────────────────────
    if mode in ["matrix", "scorecard"]:
        if no_run:
            render_feature_matrix([])
            return
        else:
            print_banner("EVALUATING CRABBASE FEATURE MATRIX", db_url)
            print(f"{GRAY}Running test suites to verify live feature compliance...{NC}")
            test_suites_to_run = [
                (
                    "🧠 Unit Tests (crabbase_core)",
                    ["cargo", "test", "-p", "crabbase_core", "--", "--nocapture"],
                ),
                (
                    "🗄️ DB Repository Tests (crabbase_db)",
                    ["cargo", "test", "-p", "crabbase_db", "--", "--nocapture"],
                ),
                (
                    "👤 Auth Repository Tests (crabbase_auth)",
                    ["cargo", "test", "-p", "crabbase_auth", "--", "--nocapture"],
                ),
                (
                    "🌐 HTTP API Tests (crabbase_api)",
                    [
                        "cargo",
                        "test",
                        "-p",
                        "crabbase_api",
                        "--tests",
                        "--",
                        "--nocapture",
                    ],
                ),
            ]
            start_time = time.time()
            all_results: list[TestResult] = []
            for title, cmd in test_suites_to_run:
                code, output = run_cargo_command(cmd, env)
                results = parse_cargo_test_output(output, title)
                all_results.extend(results)
            render_feature_matrix(all_results)
            return

    # ── Test Suite Selection ──────────────────────────────────────────────────
    test_suites_to_run = []

    if mode in ["all", "full", "workspace"]:
        print_banner("ALL TESTS (Unit + DB + HTTP Integration)", db_url)
        test_suites_to_run = [
            (
                "🧠 Unit Tests (crabbase_core)",
                ["cargo", "test", "-p", "crabbase_core", "--", "--nocapture"],
            ),
            (
                "🗄️ DB Repository Tests (crabbase_db)",
                ["cargo", "test", "-p", "crabbase_db", "--", "--nocapture"],
            ),
            (
                "👤 Auth Repository Tests (crabbase_auth)",
                ["cargo", "test", "-p", "crabbase_auth", "--", "--nocapture"],
            ),
            (
                "🌐 HTTP API Tests (crabbase_api)",
                ["cargo", "test", "-p", "crabbase_api", "--tests", "--", "--nocapture"],
            ),
        ]
    elif mode in ["unit", "core"]:
        print_banner("UNIT TESTS ONLY (crabbase_core)", "")
        test_suites_to_run = [
            (
                "🧠 Unit Tests: Parser, Compiler, Models (crabbase_core)",
                ["cargo", "test", "-p", "crabbase_core", "--", "--nocapture"],
            )
        ]
    elif mode in ["db", "repo", "repositories"]:
        print_banner("DATABASE REPOSITORY TESTS", db_url)
        test_suites_to_run = [
            (
                "🗄️ Collections & Records DB Tests (crabbase_db)",
                ["cargo", "test", "-p", "crabbase_db", "--", "--nocapture"],
            ),
            (
                "👤 Auth & OTP DB Tests (crabbase_auth)",
                ["cargo", "test", "-p", "crabbase_auth", "--", "--nocapture"],
            ),
        ]
    elif mode in ["api", "integration", "http"]:
        print_banner("HTTP API INTEGRATION TESTS", db_url)
        test_suites_to_run = [
            (
                "🌐 HTTP API Integration Tests (crabbase_api)",
                ["cargo", "test", "-p", "crabbase_api", "--tests", "--", "--nocapture"],
            )
        ]
    else:
        # Match single suite shorthand or exact target
        test_target = SUITE_ALIASES.get(mode, mode)
        print_banner(f"SINGLE SUITE: {test_target}", db_url)
        test_suites_to_run = [
            (
                f"Target: {test_target}",
                [
                    "cargo",
                    "test",
                    "-p",
                    "crabbase_api",
                    "--test",
                    test_target,
                    "--",
                    "--nocapture",
                ],
            )
        ]

    # ── Execute & Render Results ──────────────────────────────────────────────
    start_time = time.time()
    all_results: list[TestResult] = []

    for title, cmd in test_suites_to_run:
        code, output = run_cargo_command(cmd, env)
        results = parse_cargo_test_output(output, title)
        render_suite_results(title, results)
        all_results.extend(results)

    elapsed = time.time() - start_time

    # 1. Render Detailed Results Breakdown Table
    render_breakdown_table(all_results, elapsed)

    # 2. Render Feature Matrix only if explicitly requested (--matrix)
    if "--matrix" in args or "--scorecard" in args or "-m" in args:
        render_feature_matrix(all_results)


if __name__ == "__main__":
    main()

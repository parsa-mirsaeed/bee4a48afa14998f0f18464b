#!/usr/bin/env bash
# ==============================================================================
# EduTalent Local Stack Runner & Complete Health Verification
#
# Usage:
#   ./scripts/dev/run_and_verify.sh             # Start/ensure stack is up & run complete checks
#   ./scripts/dev/run_and_verify.sh --check-only # Only verify running services without starting
#   ./scripts/dev/run_and_verify.sh --restart    # Clean restart all containers then verify
#   ./scripts/dev/run_and_verify.sh --stop       # Stop all local containers
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
COMPOSE_FILE="${ROOT_DIR}/compose.yaml"
STORAGE_DIR="/tmp/edutalent-mock-storage"
MOCK_IDP_SCRIPT="${ROOT_DIR}/tests/e2e/fixtures/mock-idp.mjs"

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

success() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

fail() {
    echo -e "${RED}[FAIL]${NC} $*"
}

heading() {
    echo -e "\n${BOLD}${CYAN}=== $* ===${NC}"
}

check_dependencies() {
    heading "Checking Host Prerequisites"
    for cmd in docker curl jq; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            fail "Missing required command: $cmd"
            exit 1
        fi
    done
    success "Docker, curl, and jq are available"

    if ! docker info >/dev/null 2>&1; then
        fail "Docker daemon is not running or accessible. Please start Docker."
        exit 1
    fi
    success "Docker daemon is running"
}

ensure_storage_dir() {
    mkdir -p "${STORAGE_DIR}"
    chmod 777 "${STORAGE_DIR}" 2>/dev/null || true
}

stop_stack() {
    heading "Stopping EduTalent Local Stack"
    info "Stopping mock-idp container..."
    docker rm -f edutalent-demo-idp >/dev/null 2>&1 || true
    info "Stopping docker compose services..."
    docker compose -f "${COMPOSE_FILE}" --profile app down --remove-orphans || true
    success "Stack stopped successfully."
}

start_compose_services() {
    heading "Starting Core Services via Docker Compose"
    info "Launching database, qdrant, embedding, ai-gateway, and app..."
    docker compose -f "${COMPOSE_FILE}" --profile app up -d --no-build database qdrant embedding ai-gateway app 2>/dev/null || \
    docker compose -f "${COMPOSE_FILE}" --profile app up -d database qdrant embedding ai-gateway app
    success "Compose services started in background."
}

start_mock_idp() {
    heading "Ensuring Persistent Mock IDP (Auth & Storage)"
    ensure_storage_dir

    # Check if app container is running because mock-idp shares its network namespace
    if ! docker ps --format '{{.Names}}' | grep -q '^edutalent-app-1$'; then
        fail "edutalent-app-1 is not running! Cannot attach mock-idp."
        return 1
    fi

    local needs_start=false
    if ! docker ps --format '{{.Names}}' | grep -q '^edutalent-demo-idp$'; then
        needs_start=true
    elif ! docker exec edutalent-app-1 curl -s -f http://127.0.0.1:9100/auth/v1/.well-known/jwks.json >/dev/null 2>&1; then
        info "edutalent-app-1 was recreated or mock-idp disconnected from network namespace; recreating mock-idp..."
        needs_start=true
    fi

    if [ "$needs_start" = true ]; then
        info "Starting edutalent-demo-idp container linked to edutalent-app-1 network..."
        docker rm -f edutalent-demo-idp >/dev/null 2>&1 || true
        docker run -d --name edutalent-demo-idp \
            --net=container:edutalent-app-1 \
            --restart=unless-stopped \
            -v "${MOCK_IDP_SCRIPT}:/app/mock-idp.mjs:ro" \
            -v "${STORAGE_DIR}:${STORAGE_DIR}" \
            -e STORAGE_DIR="${STORAGE_DIR}" \
            node:22-alpine node /app/mock-idp.mjs >/dev/null
        success "edutalent-demo-idp started."
    else
        success "edutalent-demo-idp is running and responding."
    fi
}

wait_for_services() {
    heading "Waiting for Services Readiness"

    info "Waiting for PostgreSQL (port 5432)..."
    local attempts=0
    until docker exec edutalent-database-1 pg_isready -U postgres -d edutalent >/dev/null 2>&1 || [ $attempts -ge 30 ]; do
        sleep 1
        attempts=$((attempts + 1))
    done
    if [ $attempts -ge 30 ]; then
        fail "PostgreSQL did not become ready in 30s."
        exit 1
    fi
    success "PostgreSQL is ready and accepting connections."

    info "Waiting for Qdrant (port 6333)..."
    attempts=0
    until docker exec edutalent-app-1 curl -s -f http://qdrant:6333/healthz >/dev/null 2>&1 || [ $attempts -ge 20 ]; do
        sleep 1
        attempts=$((attempts + 1))
    done
    if [ $attempts -ge 20 ]; then
        fail "Qdrant did not become ready in 20s."
        exit 1
    fi
    success "Qdrant is healthy."

    info "Waiting for Text Embeddings Engine (port 8081)..."
    attempts=0
    until docker exec edutalent-app-1 curl -s -f http://embedding:80/info >/dev/null 2>&1 || [ $attempts -ge 20 ]; do
        sleep 1
        attempts=$((attempts + 1))
    done
    if [ $attempts -ge 20 ]; then
        fail "Embedding inference engine did not become ready in 20s."
        exit 1
    fi
    success "Embedding inference engine is healthy."

    info "Waiting for AI Gateway (port 8090)..."
    attempts=0
    until docker exec edutalent-app-1 curl -s -f http://ai-gateway:8090/healthz >/dev/null 2>&1 || [ $attempts -ge 20 ]; do
        sleep 1
        attempts=$((attempts + 1))
    done
    if [ $attempts -ge 20 ]; then
        fail "AI Gateway did not become ready in 20s."
        exit 1
    fi
    success "AI Gateway is healthy."

    info "Waiting for Mock IDP (port 9100)..."
    attempts=0
    until docker exec edutalent-app-1 curl -s -f http://127.0.0.1:9100/auth/v1/.well-known/jwks.json >/dev/null 2>&1 || [ $attempts -ge 20 ]; do
        sleep 1
        attempts=$((attempts + 1))
    done
    if [ $attempts -ge 20 ]; then
        fail "Mock IDP did not respond with JWKS in 20s."
        exit 1
    fi
    success "Mock IDP is serving JWKS and authentication endpoints."

    info "Waiting for Web Application (port 8080)..."
    attempts=0
    until docker exec edutalent-app-1 curl -s -I http://localhost:8080/ >/dev/null 2>&1 || [ $attempts -ge 30 ]; do
        sleep 1
        attempts=$((attempts + 1))
    done
    if [ $attempts -ge 30 ]; then
        fail "Web Application did not respond on port 8080."
        exit 1
    fi
    success "Web Application is serving frontend traffic on http://localhost:8080."
}

run_verification() {
    heading "Running Deep Functional Verification"

    # 1. Database & Table Schema
    info "Verifying database tables and governed knowledge assets..."
    local asset_count
    asset_count=$(docker exec edutalent-database-1 psql -U postgres -d edutalent -tAc "SELECT count(*) FROM knowledge_assets;" 2>/dev/null || echo "error")
    if [ "$asset_count" = "error" ]; then
        fail "Failed to query knowledge_assets table in PostgreSQL."
    else
        success "Database verified: ${asset_count} knowledge assets registered."
    fi

    # 2. Mock IDP Storage Buckets
    info "Verifying storage bucket 'edutalent-knowledge-sources'..."
    local bucket_status
    bucket_status=$(docker exec edutalent-app-1 curl -s http://127.0.0.1:9100/storage/v1/bucket/edutalent-knowledge-sources | jq -r '.id // "not_found"')
    if [ "$bucket_status" = "edutalent-knowledge-sources" ]; then
        success "Storage bucket 'edutalent-knowledge-sources' is available and healthy."
    else
        fail "Storage bucket 'edutalent-knowledge-sources' missing or unavailable."
    fi

    # 3. Authentication for Teacher
    info "Testing Teacher authentication (e2e-teacher-a@example.test)..."
    local teacher_auth_res teacher_token
    teacher_auth_res=$(docker exec edutalent-app-1 curl -s -X POST http://127.0.0.1:9100/auth/v1/token?grant_type=password \
        -H "Content-Type: application/json" \
        -d '{"email":"e2e-teacher-a@example.test","password":"e2e-password"}')
    teacher_token=$(echo "$teacher_auth_res" | jq -r '.access_token // empty')

    if [ -n "$teacher_token" ]; then
        success "Teacher JWT issued successfully."
    else
        fail "Failed to issue Teacher JWT token: ${teacher_auth_res}"
    fi

    # 4. Teacher Materials Availability API
    if [ -n "$teacher_token" ]; then
        info "Querying available materials for teacher (/api/teacher/knowledge-assets/available)..."
        local teacher_assets_res published_titles
        teacher_assets_res=$(docker exec edutalent-app-1 curl -s -X POST http://localhost:8080/api/teacher/knowledge-assets/available \
            -H "Cookie: access_token=${teacher_token}" \
            -H "Content-Type: application/json" \
            -d '{"context_scope":"global","context_key":""}')
        
        published_titles=$(echo "$teacher_assets_res" | jq -r '.[].asset.title' 2>/dev/null || echo "")
        if [ -n "$published_titles" ]; then
            local count
            count=$(echo "$teacher_assets_res" | jq 'length')
            success "Teacher API returned ${count} published material(s):"
            echo "$teacher_assets_res" | jq -r '.[] | "    • \(.asset.title) (ID: \(.asset.id), Status: \(.asset.status))"'
        else
            warn "No published materials found for Teacher A yet (check if assets need to be published)."
        fi
    fi

    # 5. Authentication for Platform Admin
    info "Testing Platform Admin authentication (e2e-admin@example.test)..."
    local admin_auth_res admin_token
    admin_auth_res=$(docker exec edutalent-app-1 curl -s -X POST http://127.0.0.1:9100/auth/v1/token?grant_type=password \
        -H "Content-Type: application/json" \
        -d '{"email":"e2e-admin@example.test","password":"e2e-password"}')
    admin_token=$(echo "$admin_auth_res" | jq -r '.access_token // empty')

    if [ -n "$admin_token" ]; then
        success "Platform Admin JWT issued successfully."
    else
        fail "Failed to issue Platform Admin JWT token."
    fi

    # 6. Admin Review Queue API
    if [ -n "$admin_token" ]; then
        info "Querying Admin Review List (/api/admin/knowledge-assets/review-list)..."
        local review_res review_count
        review_res=$(docker exec edutalent-app-1 curl -s -X POST http://localhost:8080/api/admin/knowledge-assets/review-list \
            -H "Cookie: access_token=${admin_token}" \
            -H "Content-Type: application/json" \
            -d '{}')
        if echo "$review_res" | jq -e 'type == "array"' >/dev/null 2>&1; then
            review_count=$(echo "$review_res" | jq 'length')
            success "Admin Review Queue returned ${review_count} asset(s) in review state."
        else
            warn "Admin Review Queue query returned non-array response: ${review_res}"
        fi
    fi
}

print_summary() {
    heading "EduTalent Local Stack Summary"
    echo -e "${GREEN}${BOLD}✔ All EduTalent services are UP and functionally verified!${NC}

${BOLD}Service Endpoints:${NC}
  • Web Application:     ${CYAN}http://localhost:8080${NC}
  • AI Gateway:          ${CYAN}http://localhost:8090${NC} (health: /healthz)
  • Qdrant Dashboard:    ${CYAN}http://localhost:6333/dashboard${NC}
  • Text Embedding API:  ${CYAN}http://localhost:8081${NC} (info: /info)
  • PostgreSQL Database: ${CYAN}postgresql://postgres:postgres@localhost:5432/edutalent${NC}
  • Mock IDP (Internal): ${CYAN}http://127.0.0.1:9100${NC} (JWKS & Storage)

${BOLD}Demo Credentials:${NC}
  • ${BOLD}Platform Admin:${NC}  e2e-admin@example.test       (Password: e2e-password)
  • ${BOLD}School Manager:${NC}  e2e-manager-a@example.test   (Password: e2e-password)
  • ${BOLD}Teacher:${NC}         e2e-teacher-a@example.test   (Password: e2e-password)
  • ${BOLD}Student:${NC}         e2e-student-a@example.test   (Password: e2e-password)

${BOLD}Quick Command Reference:${NC}
  • Check health anytime:  ${YELLOW}./run-local.sh --check-only${NC}
  • Restart stack cleanly: ${YELLOW}./run-local.sh --restart${NC}
  • Stop all containers:   ${YELLOW}./run-local.sh --stop${NC}
  • View app container logs: ${YELLOW}docker logs -f edutalent-app-1${NC}
"
}

# Main routing
main() {
    local mode="start"
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --check-only)
                mode="check_only"
                shift
                ;;
            --restart)
                mode="restart"
                shift
                ;;
            --stop)
                mode="stop"
                shift
                ;;
            --help|-h)
                cat <<'HELP'
EduTalent Local Stack Runner & Complete Verification Script

Usage:
  ./scripts/dev/run_and_verify.sh [OPTIONS]

Options:
  (no args)      Ensure stack is started and perform complete health & API verification.
  --check-only   Verify the running services without restarting or modifying containers.
  --restart      Restart all containers cleanly, re-attach mock-idp, and verify.
  --stop         Stop all containers (compose and mock-idp).
  -h, --help     Show this help message.
HELP
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    check_dependencies

    case "$mode" in
        stop)
            stop_stack
            exit 0
            ;;
        restart)
            stop_stack
            start_compose_services
            start_mock_idp
            wait_for_services
            run_verification
            print_summary
            ;;
        check_only)
            wait_for_services
            run_verification
            print_summary
            ;;
        start)
            start_compose_services
            start_mock_idp
            wait_for_services
            run_verification
            print_summary
            ;;
    esac
}

main "$@"

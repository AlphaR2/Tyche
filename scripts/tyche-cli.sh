#!/bin/bash
# Tyche Dev Suite - Enhanced Interactive UI
# Professional CLI for building, testing, and managing the Tyche Protocol

set -e

# Enhanced color palette
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# Box drawing characters
BOX_H="━"
BOX_V="┃"
BOX_TL="┏"
BOX_TR="┓"
BOX_BL="┗"
BOX_BR="┛"

# Function to draw a fancy box
draw_box() {
    local title="$1"
    local width=60
    local padding=$(( (width - ${#title} - 2) / 2 ))

    echo -e "${CYAN}${BOX_TL}$(printf '%*s' $width | tr ' ' "$BOX_H")${BOX_TR}${NC}"
    printf "${CYAN}${BOX_V}${NC}%*s${BOLD}%s${NC}%*s${CYAN}${BOX_V}${NC}\n" $padding "" "$title" $((width - padding - ${#title})) ""
    echo -e "${CYAN}${BOX_BL}$(printf '%*s' $width | tr ' ' "$BOX_H")${BOX_BR}${NC}"
}

# Print fancy header
print_header() {
    clear
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${BOLD}        TYCHE DEVELOPMENT SUITE           ${NC}${CYAN}║${NC}"
    echo -e "${CYAN}╠════════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${CYAN}║${NC} ${DIM}Competitive Price Discovery Engine ${NC}      ${CYAN}║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    if [ -n "$1" ]; then
        echo -e "${MAGENTA}▶ $1${NC}"
        echo ""
    fi
}

# Section header
print_section() {
    echo ""
    echo -e "${BLUE}┌─────────────────────────────────────────────────────────────┐${NC}"
    echo -e "${BLUE}│${NC} ${BOLD}$1${NC}"
    echo -e "${BLUE}└─────────────────────────────────────────────────────────────┘${NC}"
}

# Status messages
print_success() {
    echo -e "${GREEN}[DONE]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_building() {
    echo -e "${CYAN}[BUILD]${NC} $1"
}

print_testing() {
    echo -e "${MAGENTA}[TEST]${NC} $1"
}

# Configuration
CUSTOM_RPC=""

# Main Menu
show_main_menu() {
    print_header "Main Navigation"

    echo -e "${BOLD}1. Build Phase:${NC}"
    echo -e "  ${CYAN}1${NC} │ ${BOLD}Build All${NC}             ${DIM}IDL -> Codama -> SBF Pipeline${NC}"
    echo -e "  ${CYAN}2${NC} │ ${BOLD}Generate IDLs${NC}         ${DIM}Regenerate Shank IDL files${NC}"
    echo -e "  ${CYAN}3${NC} │ ${BOLD}Client Codegen${NC}        ${DIM}Generate TS client (Codama)${NC}"
    echo -e "  ${CYAN}4${NC} │ ${BOLD}Compile SBF${NC}           ${DIM}Run cargo build-sbf${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "${BOLD}2. Configuration & Setup:${NC}"
    if [ -n "$CUSTOM_RPC" ]; then
        echo -e "  ${CYAN}5${NC} │ ${BOLD}Set Custom RPC${NC}        ${DIM}Current: ${GREEN}${CUSTOM_RPC:0:30}...${NC}"
    else
        echo -e "  ${CYAN}5${NC} │ ${BOLD}Set Custom RPC${NC}        ${DIM}Current: ${YELLOW}Default (Devnet Public)${NC}"
    fi
    echo -e "  ${CYAN}6${NC} │ ${BOLD}Setup Test Env${NC}        ${DIM}Keypairs + Airdrops + NPM Install${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "${BOLD}3. Testing:${NC}"
    echo -e "  ${CYAN}7${NC} │ ${BOLD}Run All Tests${NC}         ${DIM}Run all TS integration tests${NC}"
    echo -e "  ${CYAN}8${NC} │ ${BOLD}Selective Tests${NC}       ${DIM}Pick individual program tests${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "${BOLD}Utilities:${NC}"
    echo -e "  ${CYAN}9${NC} │ ${BOLD}Clear Test Env${NC}        ${DIM}Delete .env.test and keys${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..60} )${NC}"
    echo -e "  ${RED}0${NC} │ ${BOLD}Exit${NC}"
    echo ""
    echo -ne "${BOLD}Choice:${NC} "
}

# Build Pipeline
build_all() {
    print_section "Executing Full Build Pipeline"
    
    generate_idls || return 1
    generate_client || return 1
    compile_sbf || return 1
    
    echo ""
    print_success "Full protocol build complete!"
}

generate_idls() {
    print_section "Generating Shank IDLs"
    local programs=("tyche-core" "tyche-escrow" "tyche-auction" "tyche-voter-weight-plugin")
    
    for prog in "${programs[@]}"; do
        print_building "Processing $prog..."
        shank idl --crate-root "programs/$prog" --out-dir clients/idls
    done
    
    print_info "Applying IDL fixes..."
    npx ts-node scripts/fix-idls.ts
    print_success "IDLs generated and fixed."
}

generate_client() {
    print_section "Codama Client Generation"
    print_building "Running Codama..."
    npx ts-node scripts/codama.ts
    print_building "Restoring package names..."
    node scripts/restore-pkg-names.cjs
    print_success "TypeScript client ready."
}

compile_sbf() {
    print_section "Cargo SBF Compilation"
    print_building "Building programs..."
    cargo build-sbf
    print_success "SBF binaries compiled."
}

# Load nvm so all npm calls in this session use the Linux-native Node.js.
# Must be done once at the top-level, before any subshell npm calls.
_load_nvm() {
    local nvm_dir="${HOME}/.nvm"
    if [[ -s "${nvm_dir}/nvm.sh" ]]; then
        # shellcheck source=/dev/null
        \. "${nvm_dir}/nvm.sh"
        nvm use default 2>/dev/null || nvm use node 2>/dev/null || true
    fi
    local npm_bin
    npm_bin="$(command -v npm 2>/dev/null || true)"
    if [[ "${npm_bin}" == /mnt/c/* ]]; then
        print_error "npm is still resolving to Windows: ${npm_bin}"
        print_error "Run  nvm alias default 24 && nvm use default  in your WSL terminal first."
        return 1
    fi
    print_info "npm: ${npm_bin}  ($(npm --version 2>/dev/null))"
}

# Testing
setup_env() {
    print_section "Test Environment Setup"

    # Check if clients are generated
    if [ ! -d "clients/js/src/generated/tyche-core" ]; then
        print_error "Generated clients not found. Run 'Build All' or 'Client Codegen' first."
        return 1
    fi

    _load_nvm || return 1

    if [ -n "$CUSTOM_RPC" ]; then
        print_info "Using Custom RPC: $CUSTOM_RPC"
        export RPC_URL="$CUSTOM_RPC"
    else
        print_warning "No custom RPC set. public devnet may rate-limit airdrops."
    fi

    print_info "Step 1/3: Bootstrapping Solana environment..."
    bash scripts/setup-test-env.sh

    print_info "Step 2/3: Installing SDK dependencies..."
    (cd packages/sdk && npm install)

    print_info "Step 3/3: Installing Test suite dependencies..."
    (cd tests/ts && npm install)

    print_success "Environment ready. All dependencies linked."
}

clear_env() {
    print_section "Environment Cleanup"
    print_warning "This will delete tests/ts/.env.test and ~/.config/solana/tyche-test/"
    echo -ne "${BOLD}Are you sure? (y/n):${NC} "
    read confirm
    if [ "$confirm" == "y" ]; then
        rm -f tests/ts/.env.test
        rm -rf ~/.config/solana/tyche-test/
        CUSTOM_RPC=""
        print_success "Configuration cleared."
    else
        print_info "Cleanup cancelled."
    fi
}

run_selective_tests() {
    print_header "Selective Test Runner"
    
    echo -e "${BOLD}Select program to test:${NC}"
    echo ""
    echo -e "  ${CYAN}1${NC} │ Tyche Core"
    echo -e "  ${CYAN}2${NC} │ Tyche Escrow"
    echo -e "  ${CYAN}3${NC} │ Tyche Auction"
    echo -e "  ${CYAN}4${NC} │ Voter Weight Plugin"
    echo -e "  ${CYAN}5${NC} │ SDK Flows & PDAs"
    echo -e "  ${CYAN}6${NC} │ ${BOLD}Auction Governance Demo${NC}  ${DIM}Full auction flow simulation${NC}"
    echo -e "  ${BLUE}──┼$( printf '%.0s─' {1..40} )${NC}"
    echo -e "  ${YELLOW}0${NC} │ Back"
    echo ""
    echo -ne "${BOLD}Choice:${NC} "
    read tchoice
    
    case $tchoice in
        1) (cd tests/ts && npx vitest run programs/tyche-core.test.ts) ;;
        2) (cd tests/ts && npx vitest run programs/tyche-escrow.test.ts) ;;
        3) (cd tests/ts && npx vitest run programs/tyche-auction.test.ts) ;;
        4) (cd tests/ts && npx vitest run programs/tyche-voter-weight-plugin.test.ts) ;;
        5) (cd tests/ts && npx vitest run sdk/) ;;
        6) (cd tests/ts && npx vitest run simulation/auction-governance-demo.test.ts) ;;
        0) return ;;
        *) print_error "Invalid choice"; sleep 1; run_selective_tests ;;
    esac
}

# Main Execution
main() {
    while true; do
        show_main_menu
        read choice
        
        case $choice in
            1) build_all ;;
            2) generate_idls ;;
            3) generate_client ;;
            4) compile_sbf ;;
            5) 
                print_section "RPC Configuration"
                echo -ne "${BOLD}Enter Custom RPC URL (leave empty to reset to default):${NC} "
                read rpc_input
                CUSTOM_RPC="$rpc_input"
                if [ -n "$CUSTOM_RPC" ]; then
                    print_success "RPC set to: $CUSTOM_RPC"
                else
                    print_info "RPC reset to default devnet."
                fi
                ;;
            6) setup_env ;;
            7) print_section "Running All Tests"; (cd tests/ts && npx vitest run) ;;
            8) run_selective_tests ;;
            9) clear_env ;;
            0) clear; exit 0 ;;
            *) print_error "Invalid option"; sleep 1 ;;
        esac
        
        echo ""
        echo -ne "${DIM}Press Enter to continue...${NC}"
        read
    done
}

# Check directory
if [ ! -d "programs" ]; then
    print_error "Must run from Tyche root directory."
    exit 1
fi

main

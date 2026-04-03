#!/usr/bin/env bash
set -euo pipefail
BASE=${1:-http://localhost:8080}
PASS=0; FAIL=0
check() { local name="$1" url="$2" expect="$3"; r=$(curl -s -o /dev/null -w "%{http_code}" "$url"); if [ "$r" = "$expect" ]; then echo "PASS $name ($r)"; ((PASS++)); else echo "FAIL $name (got $r, expected $expect)"; ((FAIL++)); fi; }
check "Health" "$BASE/health" "200"
check "License" "$BASE/license" "200"
check "Auth required" "$BASE/api/v1/lb/health" "401"
check "Admin forbidden" "$BASE/api/v1/admin/stats" "401"
echo "---"
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1

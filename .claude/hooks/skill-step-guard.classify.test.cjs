// Stack-agnostic classification tests for skill-step-guard.cjs (round-1 M2).
//
// Not registered as a Claude Code hook and NOT bundled by masday-cli/build.rs
// (the project-hooks copy filter only matches masday-*.{cjs,js}, run.sh, or the
// exact filename "skill-step-guard.cjs"). Run manually:
//
//     node .claude/hooks/skill-step-guard.classify.test.cjs
//
// There is no JS test runner wired into CI for hooks today; this file pins the
// classification contract for regression protection and documents the M2 fix.

const {
  getSourceExtension,
  isTestFile,
  isSourceCodeFile,
  SOURCE_EXTENSIONS,
} = require("./skill-step-guard.cjs");

let passed = 0;
let failed = 0;
function assert(name, actual, expected) {
  const ok = actual === expected;
  if (ok) {
    passed++;
  } else {
    failed++;
    console.error(`  FAIL: ${name}\n        expected ${JSON.stringify(expected)}\n        got      ${JSON.stringify(actual)}`);
  }
}
function assertTrue(name, actual) {
  assert(name, Boolean(actual), true);
}
function assertFalse(name, actual) {
  assert(name, Boolean(actual), false);
}

// ── getSourceExtension ──────────────────────────────────────────────────────
assert("rs ext", getSourceExtension("src/lib.rs"), ".rs");
assert("py ext", getSourceExtension("app/services/auth.py"), ".py");
assert("go ext", getSourceExtension("cmd/main.go"), ".go");
assert("tsx ext", getSourceExtension("src/Button.tsx"), ".tsx");
assert("ts ext (regression)", getSourceExtension("src/index.ts"), ".ts");
assert("php ext", getSourceExtension("app/Http/Controllers/User.php"), ".php");
assert("vue ext", getSourceExtension("src/App.vue"), ".vue");
assert("uppercase ext normalized", getSourceExtension("SRC/LIB.RS"), ".rs");
assertEmpty("md is not source ext", getSourceExtension("README.md"));
assertEmpty("json is not source ext", getSourceExtension("package.json"));
assertEmpty("toml is not source ext", getSourceExtension("Cargo.toml"));
assertEmpty("no ext", getSourceExtension("Dockerfile"));
// .cc / .cpp / .h must not collide with .c (dot-prefix makes them distinct)
assert("cc ext", getSourceExtension("foo.cc"), ".cc");
assert("cpp ext", getSourceExtension("foo.cpp"), ".cpp");
assert("h ext", getSourceExtension("foo.h"), ".h");
assert("hpp ext (not collapsed to .h)", getSourceExtension("foo.hpp"), ".hpp");

// ── isTestFile ───────────────────────────────────────────────────────────────
assertTrue("ts infix .test.", isTestFile("src/foo.test.ts"));
assertTrue("ts infix .spec.", isTestFile("src/foo.spec.tsx"));
assertTrue("go suffix _test.go", isTestFile("pkg/foo_test.go"));
assertTrue("rust suffix _test.rs", isTestFile("tests/parser_test.rs"));
assertTrue("py suffix _test.py", isTestFile("tests/calc_test.py"));
assertTrue("py prefix test_foo.py", isTestFile("tests/test_auth.py"));
assertTrue("jest prefix test-foo", isTestFile("test-foo.js"));
assertTrue("backslash path", isTestFile("src\\foo.test.ts"));
assertFalse("rs source is not test", isTestFile("src/lib.rs"));
assertFalse("py source is not test", isTestFile("app/auth.py"));
assertFalse("md is not test", isTestFile("docs/README.md"));

// ── isSourceCodeFile ─────────────────────────────────────────────────────────
assertTrue("rs is source", isSourceCodeFile("src/lib.rs"));
assertTrue("py is source", isSourceCodeFile("app/services/auth.py"));
assertTrue("go is source", isSourceCodeFile("cmd/main.go"));
assertTrue("tsx is source", isSourceCodeFile("src/Button.tsx"));
assertTrue("php is source", isSourceCodeFile("app/User.php"));
assertTrue("ts is source (regression)", isSourceCodeFile("src/index.ts"));
// THE M2 BUG: a Go/Rust/Python TEST file must NOT be classified as source,
// otherwise the RED guard would block writing the test itself.
assertFalse("go test file is NOT source", isSourceCodeFile("pkg/foo_test.go"));
assertFalse("rust test file is NOT source", isSourceCodeFile("tests/parser_test.rs"));
assertFalse("py test file is NOT source", isSourceCodeFile("tests/test_auth.py"));
assertFalse("ts test file is NOT source", isSourceCodeFile("src/foo.test.ts"));
// Non-code files must be excluded so docs/config never trip the guard.
assertFalse("md is NOT source", isSourceCodeFile("README.md"));
assertFalse("json is NOT source", isSourceCodeFile("tsconfig.json"));
assertFalse("toml is NOT source", isSourceCodeFile("Cargo.toml"));
assertFalse("yml is NOT source", isSourceCodeFile(".github/workflows/ci.yml"));

// ── SOURCE_EXTENSIONS sanity ─────────────────────────────────────────────────
assertTrue("covers masday stacks (rust/ts/py/go/php)",
  [".rs", ".ts", ".py", ".go", ".php"].every((e) => SOURCE_EXTENSIONS.includes(e)));

// helpers
function assertEmpty(name, actual) {
  assert(name, actual, "");
}

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);

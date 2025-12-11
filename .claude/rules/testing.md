---
paths: src/**/*.rs, tests/**/*.rs
---

# Testing Rules

## YAGNI Principle

- Do NOT write code "just in case" it might be needed later
- Only implement functionality that is currently required
- Remove unused methods, fields, and parameters immediately
- If a method is only used in tests, consider if it's truly necessary

## Parameterized Tests

- Use `rstest` crate for parameterized tests
- Convert multiple similar test cases into a single parameterized test
- Use `#[rstest]` with `#[case(...)]` attributes for test parameters

Example:
```rust
#[rstest]
#[case("input1", "expected1")]
#[case("input2", "expected2")]
fn test_something(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(process(input), expected);
}
```

## Test Organization

- Place unit tests in the same file as the implementation using `#[cfg(test)] mod tests`
- Use integration tests (`tests/`) only for testing multiple modules together
- Keep test names descriptive: `function_name_scenario_expected_behavior`

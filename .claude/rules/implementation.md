---
paths: src/**/*.rs
---

# Implementation Rules

## Error Handling Patterns

### When returning `Option` (not `Result`)

Use `inspect_err` + `ok()` to log the error before converting to `Option`:

```rust
// Good: Use inspect_err for logging before converting to Option
let Some(value) = fallible_operation()
    .inspect_err(|e| warn!("Operation failed: {}", e))
    .ok()
else {
    return default_value;
};

// Bad: Using match is verbose
let value = match fallible_operation() {
    Ok(v) => v,
    Err(e) => {
        warn!("Operation failed: {}", e);
        return default_value;
    }
};

```

### When returning `Result`

- Use `inspect_err` for logging without changing the error type
- Use `map_err` for error type conversion

```rust
// Good: inspect_err for logging, map_err for conversion
fn process() -> Result<Value, MyError> {
    fallible_operation()
        .inspect_err(|e| warn!("Operation failed: {}", e))
        .map_err(MyError::from)
}

// Good: Just logging, no conversion needed
fn process() -> Result<Value, SameError> {
    fallible_operation()
        .inspect_err(|e| warn!("Operation failed: {}", e))
}
```

## Early Returns with `let-else`

Use `let-else` pattern for early returns instead of nested `if let` or `match`:

```rust
// Good: let-else for early return
let Some(value) = optional_value else {
    return Error::NotFound;
};

// Bad: Nested structure
if let Some(value) = optional_value {
    // ... deep nesting
} else {
    return Error::NotFound;
}
```

## Trait Naming

Use names that describe the **role** or **behavior**, not just the data type:

```rust
// Good: Describes what the trait does
pub trait VersionResolver { ... }
pub trait PackageFetcher { ... }
pub trait ConfigProvider { ... }

// Bad: Just describes the data type
pub trait VersionCache { ... }
pub trait PackageData { ... }
pub trait Config { ... }
```

Suffix guidelines:
- `-er` suffix for traits that perform actions (Resolver, Fetcher, Provider, Handler)
- `-able` suffix for capability traits (Readable, Serializable)

## Function Naming

Avoid vague function names like `check`, `process`, `handle`. Use names that describe the **specific action**:

```rust
// Good: Describes what the function actually does
pub fn compare_version(...) -> VersionCompareResult { ... }
pub fn validate_semver(...) -> bool { ... }
pub fn fetch_latest_version(...) -> Option<String> { ... }

// Bad: Too vague
pub fn check_version(...) -> CheckResult { ... }
pub fn process_package(...) -> Result { ... }
pub fn handle_request(...) -> Response { ... }
```

## Return Type Naming

Return types should describe the **result of the operation**:

```rust
// Good: Type name reflects the operation result
pub struct VersionCompareResult { ... }  // Result of comparing versions
pub struct ParsedPackage { ... }         // Result of parsing a package

// Bad: Generic or unclear names
pub struct CheckResult { ... }
pub struct Data { ... }
```

## Type Conversion with `From` Trait

Use `From` trait for type conversions, but **only when the conversion is straightforward**:

```rust
// Good: Simple 1-to-1 mapping between variants
impl From<ParseError> for MyError {
    fn from(e: ParseError) -> Self {
        MyError::Parse(e)
    }
}

// Good: Direct field mapping
impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        Config {
            name: raw.name,
            value: raw.value,
        }
    }
}
```

**Do NOT implement `From`** when:
- Additional context is needed for the conversion (e.g., guard conditions)
- The conversion requires external state or side effects
- The mapping is not 1-to-1 (e.g., multiple source values map to one target)

```rust
// Bad: Conversion requires external context (version_exists check)
// Don't force From here - use explicit match instead
let status = match compare_result {
    CompareResult::Invalid => VersionStatus::Invalid,
    _ if !version_exists => VersionStatus::NotFound,  // External condition
    CompareResult::Latest => VersionStatus::Latest,
    // ...
};
```

# レジストリごとのバージョン比較処理の設計

## 現状分析

### 現在のアーキテクチャ

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Parser         │────▶│  diagnostics.rs  │────▶│  Diagnostic     │
│  (PackageInfo)  │     │                  │     │                 │
└─────────────────┘     └────────┬─────────┘     └─────────────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │  checker.rs      │
                        │  compare_version │
                        └────────┬─────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
           ┌──────────────┐          ┌──────────────────┐
           │ VersionStorer│          │    semver.rs     │
           │ (Cache)      │          │ compare_versions │
           └──────────────┘          │ version_matches  │
                                     └──────────────────┘
```

### 現在のバージョン比較フロー (GitHub Actions)

1. `generate_diagnostics()` がパーサーからパッケージ情報を取得
2. 各パッケージに対して `compare_version()` を呼び出し
3. `compare_version()` は:
   - `get_latest_version()` で最新バージョンを取得
   - `get_versions()` で全バージョンを取得
   - `version_matches_any()` で部分バージョンマッチング（`v6` が `v6.0.0` にマッチ）
   - `compare_versions()` でバージョン比較

### GitHub Actions のバージョン指定

- `v4` - メジャーバージョンのみ（`v4.0.0`, `v4.1.0` 等にマッチ）
- `v4.1` - メジャー.マイナー（`v4.1.0`, `v4.1.5` 等にマッチ）
- `v4.1.0` - 完全一致

### npm のバージョン範囲指定

| 記法      | 意味                 | 例                    |
|-----------|----------------------|-----------------------|
| `1.2.3`   | 完全一致             | `1.2.3` のみ          |
| `^1.2.3`  | minor/patch 変更可   | `>=1.2.3 <2.0.0`      |
| `~1.2.3`  | patch 変更のみ可     | `>=1.2.3 <1.3.0`      |
| `>1.2.3`  | より大きい           | `1.2.4`, `2.0.0` 等   |
| `>=1.2.3` | 以上                 | `1.2.3`, `1.2.4` 等   |
| `<1.2.3`  | より小さい           | `1.2.2`, `1.0.0` 等   |
| `<=1.2.3` | 以下                 | `1.2.3`, `1.2.2` 等   |
| `1.2.x`   | patch ワイルドカード | `1.2.0`, `1.2.999` 等 |
| `*`       | 任意                 | 全バージョン          |

## 問題点

1. **バージョンマッチングの違い**
   - GitHub Actions: 部分バージョンマッチング（`v6` → `v6.x.x`）
   - npm: 範囲指定（`^1.0.0` → `>=1.0.0 <2.0.0`）

2. **「最新バージョン」の意味の違い**
   - GitHub Actions: レジストリの最新バージョン
   - npm: 範囲内で満たす最新バージョン

3. **現在の実装の制約**
   - `compare_version()` と `version_matches_any()` は GitHub Actions 向けに設計
   - レジストリ固有のロジックが `semver.rs` にハードコード

## 設計案

### 案1: VersionMatcher トレイトの導入

レジストリごとにバージョンマッチングのロジックを抽象化する。

```rust
/// バージョンマッチング戦略
pub trait VersionMatcher: Send + Sync {
    /// 指定されたバージョンが利用可能なバージョンリストにマッチするか
    fn matches_any(&self, version_spec: &str, available: &[String]) -> bool;

    /// バージョン指定から比較用のバージョンを抽出
    /// npm: "^1.2.3" -> "1.2.3"
    /// GitHub Actions: "v4" -> "4.0.0"
    fn extract_base_version(&self, version_spec: &str) -> Option<String>;

    /// 範囲内で満たす最新バージョンを取得
    /// npm: "^1.2.3" に対して available から最新の満たすバージョンを返す
    /// GitHub Actions: latest をそのまま返す
    fn find_best_match(&self, version_spec: &str, available: &[String]) -> Option<String>;
}
```

```
┌─────────────────┐
│ VersionMatcher  │◀─────────────────────────────────┐
│ (trait)         │                                  │
└────────┬────────┘                                  │
         │                                           │
         ├───────────────────┬───────────────────────┤
         ▼                   ▼                       ▼
┌─────────────────┐  ┌─────────────────┐   ┌─────────────────┐
│ GitHubActions   │  │ NpmVersion      │   │ CargoVersion    │
│ VersionMatcher  │  │ Matcher         │   │ Matcher         │
└─────────────────┘  └─────────────────┘   └─────────────────┘
```

### 案2: compare_version の拡張

`compare_version()` に `RegistryType` を渡し、内部で分岐する。

```rust
pub fn compare_version<S: VersionStorer>(
    storer: &S,
    registry_type: RegistryType,  // &str から RegistryType に変更
    package_name: &str,
    current_version: &str,
) -> Result<VersionCompareResult, CacheError> {
    match registry_type {
        RegistryType::GitHubActions => compare_github_actions(storer, package_name, current_version),
        RegistryType::Npm => compare_npm(storer, package_name, current_version),
        // ...
    }
}
```

### 推奨: 案1 (VersionMatcher トレイト)

**理由:**
- 開放/閉鎖原則に従う（新しいレジストリ追加時に既存コードを変更しない）
- 各レジストリのロジックが独立してテスト可能
- 責務が明確に分離される

## 詳細設計

### 1. VersionMatcher トレイト

```rust
// src/version/matcher.rs

use crate::parser::types::RegistryType;

/// バージョン比較の結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    /// 範囲内の最新バージョンを使用中
    Satisfied,
    /// より新しいバージョンが範囲内に存在
    Upgradable { best_match: String },
    /// 指定されたバージョン/範囲に一致するものがない
    NotFound,
    /// バージョン指定が無効
    Invalid,
}

/// レジストリごとのバージョンマッチング戦略
pub trait VersionMatcher: Send + Sync {
    /// 対応するレジストリタイプ
    fn registry_type(&self) -> RegistryType;

    /// バージョン指定が利用可能なバージョンにマッチするかチェック
    fn check_version(
        &self,
        version_spec: &str,
        available_versions: &[String],
    ) -> MatchResult;

    /// バージョン指定から表示用のベースバージョンを抽出
    /// "^1.2.3" -> "1.2.3", "v4" -> "v4"
    fn display_version(&self, version_spec: &str) -> String;
}
```

### 2. GitHub Actions 用 Matcher

```rust
// src/version/matchers/github_actions.rs

pub struct GitHubActionsMatcher;

impl VersionMatcher for GitHubActionsMatcher {
    fn registry_type(&self) -> RegistryType {
        RegistryType::GitHubActions
    }

    fn check_version(
        &self,
        version_spec: &str,
        available_versions: &[String],
    ) -> MatchResult {
        // 既存の version_matches_any と compare_versions ロジックを使用
        // 部分バージョンマッチング: v6 -> v6.x.x
    }

    fn display_version(&self, version_spec: &str) -> String {
        version_spec.to_string()
    }
}
```

### 3. npm 用 Matcher

```rust
// src/version/matchers/npm.rs

pub struct NpmVersionMatcher;

impl VersionMatcher for NpmVersionMatcher {
    fn registry_type(&self) -> RegistryType {
        RegistryType::Npm
    }

    fn check_version(
        &self,
        version_spec: &str,
        available_versions: &[String],
    ) -> MatchResult {
        // npm の範囲指定を解析
        // ^, ~, >, >=, <, <=, x, * などを処理
        // node-semver crate または自前実装
    }

    fn display_version(&self, version_spec: &str) -> String {
        // "^1.2.3" -> "1.2.3" (範囲指定プレフィックスを除去)
        self.strip_range_prefix(version_spec)
    }
}
```

### 4. 更新後の compare_version

```rust
// src/version/checker.rs

pub fn compare_version<S: VersionStorer>(
    storer: &S,
    matcher: &dyn VersionMatcher,
    package_name: &str,
    current_version: &str,
) -> Result<VersionCompareResult, CacheError> {
    let registry_type = matcher.registry_type().as_str();

    let all_versions = storer.get_versions(registry_type, package_name)?;

    if all_versions.is_empty() {
        return Ok(VersionCompareResult {
            current_version: current_version.to_string(),
            latest_version: None,
            status: VersionStatus::NotInCache,
        });
    }

    let latest = storer.get_latest_version(registry_type, package_name)?;

    let status = match matcher.check_version(current_version, &all_versions) {
        MatchResult::Satisfied => VersionStatus::Latest,
        MatchResult::Upgradable { .. } => VersionStatus::Outdated,
        MatchResult::NotFound => VersionStatus::NotFound,
        MatchResult::Invalid => VersionStatus::Invalid,
    };

    Ok(VersionCompareResult {
        current_version: matcher.display_version(current_version),
        latest_version: latest,
        status,
    })
}
```

### 5. diagnostics.rs の更新

```rust
pub fn generate_diagnostics<S: VersionStorer>(
    parser: &dyn Parser,
    matcher: &dyn VersionMatcher,  // 追加
    storer: &S,
    content: &str,
) -> Vec<Diagnostic> {
    let packages = parser.parse(content).unwrap_or_default();

    packages
        .iter()
        .filter_map(|package| {
            let result = compare_version(
                storer,
                matcher,  // 追加
                &package.name,
                &package.version,
            )
            .ok()?;
            create_diagnostic(package, &result)
        })
        .collect()
}
```

## npm バージョン範囲の実装方針

### 選択肢

1. **node-semver crate を使用**
   - Pros: npm と完全互換、実装コスト低
   - Cons: 外部依存追加

2. **自前実装**
   - Pros: 依存を増やさない、必要な機能のみ実装
   - Cons: 実装コスト高、バグのリスク

### 推奨: 自前実装（最小限）

LSP の用途では完全な npm semver 互換は不要。以下の範囲指定のみサポート:

- `1.2.3` - 完全一致
- `^1.2.3` - minor/patch 変更可
- `~1.2.3` - patch 変更のみ可
- `>=1.2.3`, `>1.2.3`, `<=1.2.3`, `<1.2.3` - 比較演算子

## 実装計画

### Phase 1: VersionMatcher トレイト導入

1. `src/version/matcher.rs` に `VersionMatcher` トレイトを定義
2. `GitHubActionsMatcher` を実装（既存ロジックを移行）
3. `compare_version()` を更新
4. 既存テストが通ることを確認

### Phase 2: npm Matcher 実装

1. `NpmVersionMatcher` を実装
2. 範囲指定のパース関数を実装
3. テストケースを追加

### Phase 3: Backend 統合

1. `initialize_matchers()` を追加
2. `generate_diagnostics()` を更新
3. E2E テストを追加

## ファイル構成

```
src/version/
├── mod.rs
├── checker.rs          # compare_version (更新)
├── matcher.rs          # VersionMatcher トレイト (新規)
├── matchers/
│   ├── mod.rs
│   ├── github_actions.rs  # GitHubActionsMatcher (新規)
│   └── npm.rs             # NpmVersionMatcher (新規)
├── semver.rs           # 共通の semver ユーティリティ
├── cache.rs
├── registry.rs
└── ...
```

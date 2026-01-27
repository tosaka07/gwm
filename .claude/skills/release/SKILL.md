---
name: release
description: Release a new version with changelog generation, tagging, and push
argument-hint: <version>
disable-model-invocation: true
allowed-tools: Bash(git:*), Bash(cargo:*), Read, Edit, Write
---

# Release - バージョンリリース

新しいバージョンをリリースします。

## 使用方法

```
/release <version>
```

例: `/release 0.2.0`

## 実行手順

### 1. 事前チェック

- 未コミットの変更がないことを確認（`git status`）
- 引数がセマンティックバージョニング形式 (x.y.z) であることを確認
- 現在のバージョン（Cargo.toml）より大きいことを確認

### 2. Cargo.toml のバージョン更新

`Cargo.toml` の `version` フィールドを新しいバージョンに更新します。

### 3. CHANGELOG.md の生成・更新

直近のタグからの差分を取得します：

```bash
git log $(git describe --tags --abbrev=0)..HEAD --oneline
```

タグがない場合は全コミットを取得：

```bash
git log --oneline
```

CHANGELOG.md を以下のフォーマットで生成または先頭に追記します：

```markdown
## [x.y.z] - YYYY-MM-DD

### Added
- 新機能

### Changed
- 変更点

### Fixed
- バグ修正
```

コミットメッセージの分類ルール：
- `feat:`, `add:` → Added
- `fix:`, `bugfix:` → Fixed
- `refactor:`, `change:`, `update:`, `style:` → Changed
- `docs:` → Documentation
- `ci:`, `chore:` → 基本的に含めない（重要なものは Changed へ）

### 4. Cargo.lock の更新

```bash
cargo check
```

### 5. コミット

ステージングしてコミット：

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v{version}"
```

### 6. タグ付け

```bash
git tag -a v{version} -m "Release v{version}"
```

### 7. プッシュ

ユーザーに確認を求めてからプッシュ：

```bash
git push origin main
git push origin v{version}
```

## 注意事項

- 未コミットの変更がある場合は中断し、先にコミットするよう促す
- プッシュ前に必ず確認を求める
- エラー時はロールバック手順を案内する：
  ```bash
  git tag -d v{version}
  git reset --soft HEAD~1
  ```

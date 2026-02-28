# Muter

Windows のマイクミュートをグローバルホットキーで瞬時に切り替えるシステムトレイ常駐アプリケーション。

## 特徴

- **グローバルホットキー** — どのアプリがフォアグラウンドでも `Ctrl+Shift+M` でマイクをミュート/解除
- **システムトレイ常駐** — マイクアイコンがミュート状態をリアルタイム表示（白=オン / グレー+禁止マーク=ミュート）
- **デスクトップ通知** — 切り替え時に Windows トースト通知でフィードバック
- **ホットキーのカスタマイズ** — 設定ファイルで好きなキーの組み合わせに変更可能
- **軽量** — 単一バイナリ、追加ランタイム不要

## 動作環境

- Windows 10 / 11
- マイクデバイスが接続されていること

## インストール

### ビルド済みバイナリ

[Releases](../../releases) から `muter.exe` をダウンロードし、任意の場所に配置してください。

### ソースからビルド

```
git clone https://github.com/yourname/muter.git
cd muter
cargo build --release
```

ビルド成果物は `target/release/muter.exe` に出力されます。

**必要なもの:**
- [Rust](https://rustup.rs/) ツールチェイン（edition 2024）
- MSVC ビルドツール（Visual Studio のワークロードまたは Build Tools）

## 使い方

### 起動

`muter.exe` を実行するとシステムトレイにマイクアイコンが表示されます。コンソールウィンドウは表示されません。

### 操作

| 操作 | 方法 |
|------|------|
| ミュート切り替え | `Ctrl+Shift+M`（デフォルト） |
| ミュート切り替え | トレイアイコン右クリック → "Toggle Mute" |
| 終了 | トレイアイコン右クリック → "Quit" |

### トレイアイコンの状態

| アイコン | 状態 |
|----------|------|
| 白いマイク | マイク ON（ミュート解除） |
| グレーのマイク + 赤い禁止マーク | マイク OFF（ミュート中） |

## 設定

初回起動時に `~/.muter/settings.toml` が自動作成されます。

```toml
# Muter settings
#
# Hotkey to toggle microphone mute
# Modifiers: ctrl, shift, alt, super
# Keys: a-z, 0-9, F1-F12, Space, Tab, etc.
#
# Examples:
#   hotkey = "ctrl+shift+m"
#   hotkey = "ctrl+alt+m"
#   hotkey = "super+shift+a"
#   hotkey = "ctrl+F9"

hotkey = "ctrl+shift+m"
```

設定を変更した場合は、アプリを再起動してください。

## Windows 起動時に自動起動する

`muter.exe` のショートカットを以下のフォルダに配置してください。

```
%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
```

エクスプローラのアドレスバーに `shell:startup` と入力すると直接開けます。

## ライセンス

MIT

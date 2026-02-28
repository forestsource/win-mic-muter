# Developer Guide

Muter の内部アーキテクチャと実装の詳細。

## プロジェクト構成

```
muter/
├── Cargo.toml          # パッケージ定義・依存関係
├── Cargo.lock          # 依存関係のロックファイル
├── README.md           # 利用者向けドキュメント
├── docs/
│   └── developer.md    # 本ファイル
└── src/
    └── main.rs         # アプリケーション全体（単一ファイル構成）
```

シンプルなアプリケーションのため、ソースは `main.rs` の単一ファイルに集約している。

## アーキテクチャ概要

```
┌─────────────────────────────────────────────────┐
│                  main()                         │
│                                                 │
│  1. COM 初期化                                   │
│  2. マイクの現在状態を取得                         │
│  3. トレイアイコン・メニュー構築                    │
│  4. 設定読み込み・ホットキー登録                    │
│  5. Win32 メッセージループ                         │
│     ├── ホットキーイベント → do_toggle()           │
│     └── メニューイベント  → do_toggle() / 終了     │
└─────────────────────────────────────────────────┘

do_toggle()
  ├── toggle_mute()     Windows Audio API でミュート反転
  ├── set_icon()        アイコンを状態に応じて再描画
  └── Notification      デスクトップ通知を表示
```

## コンポーネント詳細

### 1. 設定管理 (`Settings`, `load_settings`)

- **設定ファイルパス:** `~/.muter/settings.toml`
- `dirs::home_dir()` でホームディレクトリを解決
- ファイルが存在しない場合、デフォルト値とコメント付きのテンプレートを自動生成
- パースエラー時はデフォルト値（`ctrl+shift+m`）にフォールバック

```rust
#[derive(Deserialize)]
struct Settings {
    #[serde(default = "default_hotkey")]
    hotkey: String,
}
```

### 2. アイコン生成 (`create_icon`)

外部画像アセットを持たず、32x32 RGBA ピクセルをプロシージャルに描画する。

**描画する要素:**

| 要素 | 形状 | 座標系 |
|------|------|--------|
| カプセル | 垂直ピル形状（中心 (16, 6)-(16, 12)、半径 4.5） | 点-線分間距離で判定 |
| ホルダー | U 字アーク（中心 (16, 12)、内径 5.5・外径 7.5、y >= 12） | 円環の下半分 |
| スタンド | 垂直バー（中心 x=16、幅 3、y: 19.5-25.0） | 矩形判定 |
| ベース | 水平バー（x: 10-22、y: 25.0-27.0） | 矩形判定 |

**状態による変化:**

- **ミュート解除:** 白色 `[255, 255, 255, 255]`
- **ミュート中:** グレー `[160, 160, 160, 255]` + 赤い禁止マーク
  - 赤い円（中心 (16, 16)、半径 14、線幅 3）
  - 対角スラッシュ（左上→右下、円の内側のみ）

### 3. Windows Audio API 連携

COM (Component Object Model) 経由で Windows Core Audio API を使用。

**初期化フロー:**

```
CoInitializeEx(COINIT_APARTMENTTHREADED)
    │
    ▼
CoCreateInstance(MMDeviceEnumerator)
    │
    ▼
IMMDeviceEnumerator::GetDefaultAudioEndpoint(eCapture, eConsole)
    │
    ▼
IMMDevice::Activate() → IAudioEndpointVolume
```

**主要関数:**

- `get_mic_endpoint_volume()` — デフォルトキャプチャデバイスの `IAudioEndpointVolume` を取得
- `toggle_mute(ep)` — `GetMute()` で現在値を取得し、`SetMute()` で反転

**COM スレッドモデル:** STA (Single-Threaded Apartment) を使用。Win32 メッセージループとの整合性のために `COINIT_APARTMENTTHREADED` で初期化。

### 4. システムトレイ (`tray-icon`)

`TrayIconBuilder` でトレイアイコンを構築。

- **メニュー項目:** "Toggle Mute", "Quit"
- **ツールチップ:** "Muter"
- **アイコン:** `create_icon()` の戻り値（状態変化時に `set_icon()` で差し替え）

### 5. グローバルホットキー (`global-hotkey`)

- `GlobalHotKeyManager` でホットキーを OS に登録
- 設定ファイルの文字列を `HotKey::parse()` でパース
- `GlobalHotKeyEvent::receiver()` のチャネルからイベントを受信
- `HotKeyState::Pressed` のみに反応（Released は無視）

### 6. イベントループ

Win32 メッセージポンプ (`GetMessageW` / `TranslateMessage` / `DispatchMessageW`) をメインループとして使用。

```
while GetMessageW(...) {
    TranslateMessage(...)
    DispatchMessageW(...)

    // 非ブロッキングでチャネルをポーリング
    hotkey_rx.try_recv()  → ホットキーイベント処理
    menu_rx.try_recv()    → メニューイベント処理
}
```

- シングルスレッドで動作、非同期ランタイム不使用
- `GetMessageW` がメッセージ待ちでブロックし、CPU を消費しない
- `try_recv()` で各チャネルを非ブロッキングにポーリング

### 7. デスクトップ通知 (`notify-rust`)

`Notification::new()` で Windows トースト通知を送信。

- サマリー: "Muter"
- ボディ: "Microphone Muted" / "Microphone Unmuted"

## 依存クレート

| クレート | バージョン | 用途 |
|----------|-----------|------|
| `tray-icon` | 0.21 | システムトレイアイコンとメニュー |
| `global-hotkey` | 0.7 | グローバルホットキーの登録・受信 |
| `notify-rust` | 4 | デスクトップ通知 |
| `serde` | 1 (derive) | 設定ファイルのデシリアライズ |
| `toml` | 0.8 | TOML パーサー |
| `dirs` | 6 | ホームディレクトリの取得 |
| `windows` | 0.58 | Win32 API バインディング |

**使用する Windows API フィーチャー:**

- `Win32_Foundation` — `BOOL` 型
- `Win32_Media_Audio` — `IMMDeviceEnumerator`, `MMDeviceEnumerator`, `eCapture`, `eConsole`
- `Win32_Media_Audio_Endpoints` — `IAudioEndpointVolume`
- `Win32_System_Com` — `CoInitializeEx`, `CoCreateInstance`, `CLSCTX_ALL`, `COINIT_APARTMENTTHREADED`
- `Win32_UI_WindowsAndMessaging` — `GetMessageW`, `TranslateMessage`, `DispatchMessageW`, `MSG`

## ビルド

```bash
# デバッグビルド
cargo build

# リリースビルド（最適化あり）
cargo build --release
```

`#![windows_subsystem = "windows"]` により、リリースバイナリはコンソールウィンドウを表示しない。デバッグ時に `println!` を使いたい場合は、この行をコメントアウトする。

## 設計上の判断

### 単一ファイル構成
約 200 行と小規模なため、モジュール分割のオーバーヘッドを避けて `main.rs` に集約。機能が増える場合は `audio.rs`, `tray.rs`, `settings.rs` 等への分割を検討する。

### プロシージャルアイコン
画像ファイルを同梱せず、ピクセル単位で描画することで：
- 外部アセットへの依存を排除
- ミュート状態に応じた動的な切り替えを単純化
- 単一バイナリでの配布を実現

### STA + メッセージループ
COM の STA モデルと Win32 メッセージループは自然に統合できる。マルチスレッドや非同期ランタイムを使わないことで、実装の複雑さを最小限に抑えている。

### エラーハンドリング
システムトレイ常駐アプリとして、致命的なエラー（マイクデバイスなし、COM 初期化失敗等）は起動時に `unwrap()` でパニックさせる設計。起動後の通知送信失敗など非致命的なものは `let _ =` で無視する。

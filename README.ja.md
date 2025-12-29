# back-directory (bd)

高速で正確なディレクトリのバックトラックと、直前の操作を1回で取り消せる機能を備えた
zshラッパー + Rustコアです。

English: [README.md](README.md)

## インストール

### 推奨: ワンライナー (GitHub Releases)

```zsh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/scripts/install.sh | sh
```

これはコアバイナリを `${XDG_BIN_HOME:-$HOME/.local/bin}` にインストールし（無ければ作成）、
ラッパーを `${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh` に配置して
`.zshrc` に `source "${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh"` を追記します。
`${XDG_BIN_HOME:-$HOME/.local/bin}` が `PATH` に無い場合は、シェル設定に追加してください。

```zsh
export PATH="$HOME/.local/bin:$PATH"
```

### ラッパーの読み込み

新しいシェルを開くか `source ~/.zshrc` を実行してください。

コアバイナリの場所が別なら、読み込み前に `BD_CORE_BIN` を指定します。

```zsh
export BD_CORE_BIN=/path/to/bd-core
```

## 使い方

```zsh
bd       # 同じ意味: bd 1
bd 3     # 3階層戻る (1 <= N <= 99)
bd c     # 現在のセッションで直前のbdを取り消し
```

任意のエイリアス:

```zsh
bd cancel
```

セッションについて: `bd` はセッション単位で履歴を追跡します。デフォルトでは端末の
TTY からセッションIDを作るため、新しいタブ/ウィンドウは別セッションになり、
同じタブ内で開いた新しいシェルは履歴を共有します。上書きしたい場合は
`BD_SESSION_ID` を設定してください。

```zsh
export BD_SESSION_ID=work-logs
```

## 開発者向け

実装や開発フローは `docs/development.md` を参照してください。

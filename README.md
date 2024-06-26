# Cantari / Let UTAUs speak on Voicevox

Cantariは、UTAUの歌声をVoicevoxで話させるためのエンジンです。

> [!IMPORTANT]
> 現在、単独音のみ対応しています。連続音やCVVCは未対応です。

## TODO

- [ ] 疑似疑問系
- [ ] 連続音

## インストール

1. [Releases](https://github.com/sevenc-nanashi/cantari/releases)から最新のバージョンの vvpp ファイルをダウンロードしてください。
2. Voicevoxの設定を開き、「高度な設定」から「マルチエンジン機能」を有効化してください。
3. 「エンジンの管理」/「追加」/「VVPP ファイル」からインストールしてください。
4. [`127.0.0.1:50202`](http://127.0.0.1:50202) にアクセスして、UTAU音源のパスを設定してください。

## ライセンス

MIT License で公開しています。詳しくは[LICENSE](LICENSE)をご覧ください。  
生成された音声については、音源の規約に従ってください。
このブリッジ自体にはクレジット表記は必要ありませんが、このリポジトリのリンクを貼ったり[紹介動画](https://www.nicovideo.jp/watch/sm43856969)を親作品登録していただいたりすると嬉しいです。
もしタグをつけるなら[`cantari使用`](https://nicovideo.jp/tag/cantari使用)でお願いします。（必須ではないです）

<!-- 
# 自分用メモ

## リリース手順

1. CHANGELOG.md を更新
2. `cargo test`
3. `git push origin main`
4. `gh workflow run build.yml -F version=0.0.0`

-->

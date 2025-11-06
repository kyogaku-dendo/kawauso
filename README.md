# kawauso

第51回筑波大学学園祭「雙峰祭」企画「驚額の殿堂3（スリー）」で用いられた，レシートをプリントするサービス

## prerequisites

- Rust
- Python 3.x
    - pikepdfとpyhankoパッケージ
- `lpr`コマンド

本アプリケーションは技術同人誌『フランクフルト』PDF版購入者のために，取引ごとに一意な署名付きPDFレシートを発行する機能を持っています．これを使うためには各自の環境で秘密鍵を発行してください．このとき証明書は`cert/cert.crt`に，秘密鍵は`cert/key.pem`に保存してください．

さらに`.env.sample`をコピーして`.env`を作成し，必要な環境変数を設定してください．`BASE_PDF_PATH`は署名対象となるPDFのパスです．

## 概要

`cargo run`するとRustのActix Webサーバーが起動します．これは次の4つのエンドポイントを持ちます：

- `GET /health` : ヘルスチェック用
- `POST /cut` : 紙詰まりを起こしたときのリセット用に，プリンターに感熱紙をカットさせる
- `POST /print/pdf` : 取引ごとに一意なUUIDを発行し，それを秘密鍵を用いて署名，R2にアップロードしてそのPDFへのURLが載ったレシートを発行
- `POST /print/tag` : 注文データを受け取って，そのレシートを発行

次のプリンターで動作確認をしていますが，日本語が発行できないのでこれは`img`以下の画像を用いています．この作成には`extract_text_rows.py`を用いて複数行のテキストをtesseractで抽出し，bounding boxごとに画像を切り出しています．

https://www.amazon.co.jp/dp/B0DH98QF55

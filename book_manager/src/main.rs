use std::{env, net::SocketAddr, sync::Arc};

use axum::{http::StatusCode, response::IntoResponse, routing::get, Extension, Json, Router};

/**
 * Rustで日時を扱うためのクレート
 * 日付型や日時型、タイムゾーンを考慮した日時型などさまざまな型を提供
 * serdeでchronoを変換するためにserdeというフィーチャーを指定
 */
use chrono::NaiveDateTime;
/**
 * 
 * serdeは、Rustで書いたデータ型をさまざまな形式のデータ構造に変換するクレート
 * Rustの構造体をJSONに変換する際にserdeを利用
 * 「derive」というフィーチャーは、serdeが持つアトリビュート機能をオンにしてより手軽に実装を行える
 */
use serde::Serialize;
/**
 * sqlxは、データベース接続やコネクションプールの管理、データベースへのクエリー発行などを行うクレート
 * データベースとして「MySQL」を利用するので、mysqlフィーチャーを指定
 * chronoを扱えるようにchronoフィーチャーも指定
 * 「runtime-tokio-rustls」はランタイムにtokioを使いつつ、TLSバックエンドとしてrustlsを利用
 */
use sqlx::{MySql, MySqlPool, Pool};

/**
 * 書籍情報を表す構造体
 * データベースに問い合わせた結果のデータを格納するのに使用
 * Bookには #[derive(Serialize)] というアトリビュートが付与されている。
 * これがserdeクレートによるアトリビュートだ。
 * これを付与することで、Rustの構造体をJSON形式に変換する実装を自動で導出できる。
 */
#[derive(Serialize)]
struct Book {
    id: i64,
    title: String,
    author: String,
    publisher: String,
    isbn: String,
    comment: String,
    // NativeDateTimeはchronoクレートが提供する型
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

// 書籍リストの情報を表す構造体
#[derive(Serialize)]
struct BookList(Vec<Book>);

// not connect のみを返す関数
// IntoResponseトレイトが実装された型であればどんな型でも返せる
async fn health_check() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

// MySQLへのコネクションプールを扱いやすくするための型エイリアス
type MySqlConPool = Arc<Pool<MySql>>;

// 書籍のリストを取得するAPIの実装
async fn book_list(
    // layerで登録したコネクションプール情報を取り出す
    Extension(db): Extension<MySqlConPool>,
) -> Result<impl IntoResponse, StatusCode> {
    // コネクションプールから1つのコネクションを取得
    let conn = db.acquire().await;
    if conn.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // データ取得してBook構造体にひも付け
    sqlx::query_as!(Book, "select * from books")
        .fetch_all(&mut conn.unwrap())
        .await
        // j. 取得した複数件をBookList構造体にひも付けてJSONで返す
        .map(|books| Json(BookList(books)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {

    /**
     * sqlxクレートを使ってコネクションプールを用意
     * データベースに接続し、接続を確立できた時点でコネクションプールを生成
     */
    let pool = MySqlPool::connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    /**
     * Routeの機能は、パスに対応する関数を登録しておくと、そのパスに対してリクエストが来たときに、該当する関数を自動で呼び出してくれる機能
     * GETリクエストに反応したい場合は「axum：：routing：：get」、POSTに反応したい場合は「axum：：routing：：post」を設定
     * 
     * 新しいAPIをルーターに登録し、コネクションプールを設定
     * layer(Extension(Arc::new(pool))) という記述も追加
     * これは、各APIでコネクションプールを取り出して使うために必要な記述になる
     * 
     * Arc は並行処理で安全に参照カウントを扱うための型だ
     * Rc（参照カウント） の並行処理版に相当する。
     */

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/books", get(book_list))
        .layer(Extension(Arc::new(pool)));

    // 127.0.0.1の3000番ポートのソケットを用意
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // 用意したソケットをバインドしてサーバーを立ち上げる
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
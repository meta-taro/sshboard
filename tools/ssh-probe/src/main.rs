//! Phase 0 の 002 / 003 を潰すための探り棒。**製品ではありません。**
//!
//! 同じことを `russh` と `ssh2` の両方でやり、**結果だけ**を出します。
//!
//! **接続先は出力に載せません**（PRD §8）。ホスト名・IP・利用者名・パス・
//! ディレクトリの中身・ファイルの中身は、どれも表示しません。
//! 出るのは「繋がったか」「どの方式か」「何件か」「どの文字コードか」だけです。
//!
//! **パスフレーズを引数で渡さないでください。**シェルの履歴に平文で残ります。
//! `--ask-passphrase` を付けると、その場で入力を求めます。

mod kexinit;
mod offer;
mod report;
mod sniff;
mod via_russh;
mod via_ssh2;

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Parser;
use sshboard_connections::Connections;

/// `exec` が通ったことを確かめるための固定文字列。
/// **こちらが決めた文字列なので、出力に載せても接続先が漏れない。**
pub const EXEC_MARKER: &str = "sshboard-probe-ok";

/// 認証のやり方。**パスワード認証は用意しません**（履歴に残る経路を作らない）。
pub enum Auth {
    Agent,
    Key {
        path: String,
        passphrase: Option<String>,
    },
}

#[derive(Parser)]
#[command(
    name = "sshboard-ssh-probe",
    about = "Phase 0 探り棒。russh と ssh2 の両方で実機へ繋ぎ、結果だけを出す。"
)]
struct Cli {
    /// sshboard に登録した接続の識別子。**これを使うのが本筋です。**
    ///
    /// アプリで 1 回登録すれば、以後どこにも接続先を書かずに済みます。
    #[arg(long, conflicts_with = "host")]
    connection: Option<String>,

    /// 接続一覧の置き場所。省略すると OS の既定の場所。
    #[arg(long)]
    connections_file: Option<PathBuf>,

    /// 接続先を直接指定する。**`--connection` を使えるならそちらを使ってください。**
    /// コマンドラインに書くと、シェルの履歴と `ps` の出力に残ります。
    #[arg(long, env = "SSHBOARD_PROBE_HOST", hide_env_values = true)]
    host: Option<String>,

    #[arg(long, env = "SSHBOARD_PROBE_PORT", default_value_t = 22)]
    port: u16,

    /// KEXINIT を読むだけなら不要。`--connection` を使うなら不要。
    #[arg(long, env = "SSHBOARD_PROBE_USER", hide_env_values = true)]
    user: Option<String>,

    /// 提示された方式を読むだけで終える。**認証も接続もしない。**
    /// 最初はこれで、サーバーが何を出しているかだけ見るのが安全。
    #[arg(long, default_value_t = false)]
    offer_only: bool,

    /// 秘密鍵のパス。省略すると ssh-agent を使う。**鍵そのものは読み込まれるだけで、
    /// どこにも出力されません。**
    #[arg(long, env = "SSHBOARD_PROBE_KEY", hide_env_values = true)]
    key: Option<String>,

    /// 鍵にパスフレーズがある場合に、その場で入力を求める。
    #[arg(long, default_value_t = false)]
    ask_passphrase: bool,

    /// `sftp` の `ls` を試すパス。**件数だけ出ます。**
    #[arg(
        long,
        env = "SSHBOARD_PROBE_SFTP_PATH",
        hide_env_values = true,
        default_value = "."
    )]
    sftp_path: String,

    /// 文字コードを見たいファイルのパス（ログや設定ファイル）。
    /// **中身は出ません。**判定と統計だけ出ます。
    #[arg(long, env = "SSHBOARD_PROBE_SNIFF", hide_env_values = true)]
    sniff: Option<String>,
}

/// 実際に使う接続先。**画面にも履歴にも出さない。**
struct Target {
    host: String,
    port: u16,
    user: Option<String>,
    key: Option<String>,
}

/// `--connection` が指定されていれば登録済みの一覧から、無ければ引数から組み立てる。
fn resolve_target(cli: &Cli) -> Result<Target> {
    let Some(id) = &cli.connection else {
        let Some(host) = cli.host.clone() else {
            bail!("--connection か --host のどちらかが要ります");
        };
        return Ok(Target {
            host,
            port: cli.port,
            user: cli.user.clone(),
            key: cli.key.clone(),
        });
    };

    let path = match &cli.connections_file {
        Some(path) => path.clone(),
        None => sshboard_connections::default_path()?,
    };
    let connections = Connections::load_or_empty(&path)?;

    let Some(entry) = connections.get(id) else {
        // **登録名は出してよい。**接続先ではない。
        let known: Vec<&str> = connections
            .connections
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        bail!("`{id}` という接続は登録されていません。登録済み: {known:?}");
    };

    Ok(Target {
        host: entry.host.clone(),
        port: entry.port,
        user: Some(entry.user.clone()),
        key: entry.key_path.clone().or_else(|| cli.key.clone()),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let target = resolve_target(&cli)?;

    println!("# sshboard Phase 0 探り棒\n");
    println!("接続先は出力に含まれません。**このまま貼って構いません。**\n");

    print_offer(&target).await;

    if cli.offer_only {
        return Ok(());
    }

    let Some(user) = target.user.clone() else {
        bail!("利用者名がありません。--connection か --user を指定してください");
    };

    let auth = match &target.key {
        None => Auth::Agent,
        Some(path) => {
            let passphrase = if cli.ask_passphrase {
                Some(rpassword::prompt_password("鍵のパスフレーズ: ")?)
            } else {
                None
            };
            Auth::Key {
                path: path.clone(),
                passphrase,
            }
        }
    };

    let russh_report = via_russh::run(
        &target.host,
        target.port,
        &user,
        &auth,
        &cli.sftp_path,
        cli.sniff.as_deref(),
    )
    .await;
    println!("{}", russh_report.render());

    // ssh2 は同期 API。async のスレッドを塞がないよう別スレッドへ出す。
    let ssh2_report = {
        let (host, sftp_path, sniff_path) = (
            target.host.clone(),
            cli.sftp_path.clone(),
            cli.sniff.clone(),
        );
        let port = target.port;
        tokio::task::spawn_blocking(move || {
            via_ssh2::run(&host, port, &user, &auth, &sftp_path, sniff_path.as_deref())
        })
        .await?
    };
    println!("{}", ssh2_report.render());

    println!("---\n");
    println!("**ホスト鍵の指紋を `known_hosts` と突き合わせてください。**");
    println!("この探り棒はホスト鍵を検証せずに受け入れます（製品側では受け入れません）。");

    Ok(())
}

/// サーバーが提示している方式を、ライブラリを通さずに出す。
/// **002 の「古い鍵交換方式・暗号方式が残っているか」はここに出る。**
async fn print_offer(target: &Target) {
    println!("## サーバーが提示した方式（KEXINIT を直接読んだもの）\n");

    match offer::fetch(&target.host, target.port).await {
        Ok(contact) => {
            println!("- 名乗り: `{}`", contact.banner);
            let offer = contact.offer;
            for (label, list) in [
                ("鍵交換", &offer.kex_algorithms),
                ("ホスト鍵", &offer.host_key_algorithms),
                ("暗号(c2s)", &offer.encryption_client_to_server),
                ("MAC(c2s)", &offer.mac_client_to_server),
                ("圧縮(c2s)", &offer.compression_client_to_server),
            ] {
                println!("- {label}: {}", list.join(", "));
            }
        }
        Err(error) => println!("- 読めません: {error:#}"),
    }
    println!();
}

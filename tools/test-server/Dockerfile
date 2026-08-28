# 手元で完結して試すための sshd（product-baseline §3）。
#
# **本番と同じ系統に合わせています。**実機は OpenSSH 8.7 / 9.9 の RHEL 系でした
# （Issue 002）。AlmaLinux 9 は RHEL 9 系で、OpenSSH 8.7 が載ります。
#
# **ここで再現したいのは「繋がること」ではありません。**
#   - /var/log がログインユーザーで読めないこと（D20 の setfacl を試すため）
#   - ログが UTF-8 とは限らないこと（Issue 002 の文字コード）
#   - tail -f が流れ続けること（Issue 005）
FROM almalinux:9

# glibc-gconv-extra が無いと iconv が EUC-JP を知らない（RHEL 9 は変換表が別）。
RUN dnf -y install openssh-server acl glibc-langpack-ja glibc-gconv-extra \
    && dnf clean all \
    && ssh-keygen -A

# 実機と同じく、鍵だけで入る
RUN sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config \
    && sed -i 's/^#\?PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config \
    # テストが並列に走ると既定の 10 を超える。**実際に弾かれた。**
    && printf 'MaxStartups 100:30:200\nMaxSessions 50\n' >> /etc/ssh/sshd_config

# ログインする利用者。**root ではない。**
RUN useradd -m -s /bin/bash probe

# --- ここから「再現したい状態」 ---

# 1. root しか読めないログ。実機の /var/log/maillog と同じ状態
RUN printf 'Aug 27 10:00:00 host postfix/smtpd[1]: connect from unknown\n' > /var/log/maillog \
    && chown root:root /var/log/maillog && chmod 600 /var/log/maillog

# 2. EUC-JP のログ。**UTF-8 とは限らない**ことを試すため
RUN printf '設定ファイルの読み込みに失敗しました\nメールの配送キューが滞留しています\n' \
    | iconv -f UTF-8 -t EUC-JP > /var/log/japanese-euc.log \
    && chmod 644 /var/log/japanese-euc.log

# 3. 利用者が読めるアプリのログ。**実務ではここが読めれば足りる場合がある**
RUN mkdir -p /home/probe/app/logs \
    && printf 'Aug 27 10:00:00 app started\nAug 27 10:00:01 listening\n' > /home/probe/app/logs/app.log \
    && chown -R probe:probe /home/probe/app

COPY grow.sh /usr/local/bin/grow.sh
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/grow.sh /usr/local/bin/entrypoint.sh

EXPOSE 22
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

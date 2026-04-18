world-online = world online: did={$did} endpoint={$endpoint} services={$services}
publish-ok-source = publisert did={$did} kilde={$source}
publish-ok-alias = publisert did={$did} cid={$cid} alias={$alias}
inbox-received = mottok inbox: {$from} -> {$to} type={$content_type} id={$id}
ipfs-received = mottok ipfs: {$from} -> {$to} type={$content_type} id={$id}
ipfs-reply = ipfs svar: til={$to} status={$status} code={$code} id={$id} type={$content_type}
cli-usage = Bruk: ma-world [--slug <slug>] [--status-api-bind <host:port>] [--gen-headless-config] [--kubo-rpc-api <url>]
    Standarder: --slug world, --status-api-bind 127.0.0.1:5002, --kubo-rpc-api http://127.0.0.1:5001
cli-missing-value = mangler verdi for {$flag}
cli-unknown-argument = ukjent argument: {$arg}
generated-headless-config = genererte headless world config i {$path}

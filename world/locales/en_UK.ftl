world-online = world online: did={$did} endpoint={$endpoint} services={$services}
publish-ok-source = published did={$did} source={$source}
publish-ok-alias = published did={$did} cid={$cid} alias={$alias}
inbox-received = received inbox: {$from} -> {$to} type={$content_type} id={$id}
ipfs-received = received ipfs: {$from} -> {$to} type={$content_type} id={$id}
ipfs-reply = ipfs reply: to={$to} status={$status} code={$code} id={$id} type={$content_type}
cli-usage = Usage: ma-world [--slug <slug>] [--status-api-bind <host:port>] [--gen-headless-config] [--kubo-rpc-api <url>]
    Defaults: --slug world, --status-api-bind 127.0.0.1:5002, --kubo-rpc-api http://127.0.0.1:5001
cli-missing-value = missing value for {$flag}
cli-unknown-argument = unknown argument: {$arg}
generated-headless-config = generated headless world config at {$path}

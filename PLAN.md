Vi skal lage en verden basert på actor som i Hewitts Actor modell. Det er helt grunnleggende for slt vi gjør her nu. 
Meldingene er basrt på en DID og DIDCOmm som er spesifiert i <https://github.com/bahner/ma-spec>.
ma-did <https://docs.rs/ma-did/latest/ma_did/> er en implementasjon av selve spec'en.
ma-core <https://docs.rs/ma-core/latest/ma_core/> er primitiver for å starte en tjeneste og motta meldinger over tjenester. Formatet av innholdet i disse meldinger er ikke definerte. 
Dem skal vi definerer her i spec, slik at andre kan implementere klienter mot slike verdener.

Jeg har diskutert permissions og requirements og slikt med chatgpt. Dette dokumentet kan være inspirerende. https://chatgpt.com/share/69e3409f-550c-83eb-ba38-b6742e1745e8

Vi skal nu begynne helt fra scratch. Vi kan koble til og så får vi en outbox, vi kan sende meldinger til. Den først vi skal definere er protokollen for metoden /ma/ipfs/0.0.1
Dette er en meget spesiell tjeneste som krever tillit, siden vi tar imot en hemmelig ipns nøkkel og et did dokument. Dette skal så publiseres lokalt til kubo.

Så det første vi gjøre er å lage world binæren. Den skal bruke musl og compile til en statisk binær. Vi skal støtte windows, macos, og linux musl.

Binæren skal starte opp og bruke ma-core. Den skal da opprette et endpoint ved å bruke ma-core. Den skal enable /ma/ipfs/0.0.1 service og meldingene som kommer inn i den innboks skal vi nu spekke.

Meldinger som kommer på /ma/inbox/0.0.1 skal bare skrivess til console og logges pt.

Identitet for endepunktet og slikt leses fra XDG_CONFIG_HOME/ma/<slug>.yaml som for eksempel:
```bash
lars@localhost:~/.config/ma$ cat *
---
kubo_rpc_api: http://localhost:5001
owner: did:ma:k51qzi5uqu5dglxrtfnvh2tx1wddufnscvi50zm90wyll9i3k0bkoofrb35uoc#bahner
iroh_secret: /home/lars/.config/ma/panteia_iroh.bin
log_level: info
log_file: ~/.local/share/ma/worlds/panteia/panteia.log
unlock_passphrase: bJ1VbUrtW97QxgOQDGeDQTp1PvDI9M71
unlock_bundle_file: /home/lars/.config/ma/panteia_bundle.json
{"version":1,"kdf":"argon2id","salt_b64":"eHZmDgkGoAmHmGU+WqtaDw==","nonce_b64":"MRzCMG85B8ZCHcuh9CcYqqneDF3iHl5P","ciphertext_b64":"ELXwajopU+A3ZrUX4zlYZkBJbB4DNMttEp+QHu1QQTkaxxPgYoR82qtcK9oCuE/J04NP9SCX3//RT84bUScG7bHyLsQwjiLgT9Bcj7X1csyu49+hDFdhao23uh1qIb4ou9Wo"}?�ip?]�o���9�Cϝ@�K�«{����lars@localhost:~/.config/ma$
```
Vi skal ha loglevel og logge som indikert

Vi skal nu diskutere innholdet for meldinger til /ma/ipfs/0.0.1 og publiseringen der. Kubo kjører lokalt på denne PC. Du kan anta at kubo er tilstede på den konfigurerte URL

Vi skal skrive protokollen vi kommer frem til i spec mappen. Disse specs skal offentliggjøres og være basis for fremdrift


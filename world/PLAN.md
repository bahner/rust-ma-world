Less AGENTS.md og ACTOR.md. Jeg skal ha en webapplikasjon som jeg kan bruke som actor mot world i dette repo. Jeg bruker ma-did og ma-core vrates som primitover for å oppnå dette. Vi tar det med ro. Først skal man en lokal identity som skal kunne lagres kryptert og åpnes trygt med et passord. Den skal inneholde en ipns nøkkel og en did identitet. Ref. https://github.com/bahner/ma-spec. ma-did og ma-core hjelper deg med mye av dette.

Publisering av et slikt dokument innebærer å sende den hemmelig nøkkel med over til /ma/ipfs/0.0.1 tjenesten til en verden, som så publiserer dokumentet for os. content-type er application/x-ma-ipfs-request som inneholder et did dokument application/x-ma-doc og den hemmelige ipns nøkkel, som skal brukes for å publisere dokumentet

Defor skal vi kunne sette et home. Som vi stoler på til å publisere dokumentet for oss. Vi skal kjøre som en webapplikasjon, så vi har ikke tilgang til kubo lokalte direkte, men vi kan bruke ipfs via gateways til å hente dokumenter, det være seg /ipns eller /ipfs. Dette er grunnelggende for at ma-skal funke. Ref. spec'en i github.

Alle objekter har en identitet og en url. Vi bruker url'en til å sende did meldinger til objektet postkasse.

Innstillnger lagres i en dot.notasjon både remote og lokalt. Dette skal kunne IPLD lenkes, så det er viktig å skille mellom privat og offentlig informasjon hele tiden. Til å begynne med trenger vi å sette en hjem, som vi kan publisere til og koble til via tjenesten /ma/avatar/0.0.1 som gir oss en avatar vi kan handle som i en verden.

Grensesnittet skal være et klassisk MUD-client grenssenitt med et readline input felt , et større dialog vindue og en oversikt over tilgjengelige avatarer i rommet man kobler til til høyre. De skal listes som avatar_id_fragment(avatar.identity)

Innstillinger fpr vår actor lagres lokalt som:
actor.home

Når vi skal sende meldinger til et hvilketsom helst objekt, så må vi sende det til deres did_url, eg: did:ma:identity#id

Noen spørsmål før vi starter?

Bygg en ny actor-crate med en wasm-basert webapp som følger ACTOR.md, bruker ma-did og ma-core, og leverer første vertikale slice: lokal kryptert identity, @my.home, DID-publish via world sin ma/ipfs-tjeneste, avatar-tilkobling via ma/avatar, og et klassisk MUD-grensesnitt. Løsningen bør splitte browser-spesifikk lagring/UI fra delt actor-logikk, slik at target-resolusjon, dot-notasjon og DID/DID URL-regler kan holdes deterministiske og testbare.
resultatet skal være en mappe som kan publiseres til en webserver og der kjøre appen. Subsidiært kan den kjøres lokale med make serve
Appen skal ha en skjermsparer som inviterer til å drikke en Pan-Galactic Gargle Blaster og viser ordene Don't Panic i store vennlige animerte bokstaver og en babelfisk som svømmer rolig rundt. Det skal være tilstrekkelig å klikke for å komme tilbake
LAg en makefile med standard, build, clean, distclean og server. Jeg skal selv legge til et publish target

Appen skal skrives i rust og compile til wasm. Layour og slikt skal ligge i en www mappe slikt at det er lett å inspisere HTML koden. Resultat skal være en statisk web app uten backend.


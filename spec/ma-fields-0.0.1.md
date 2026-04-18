# ma.fields/0.0.1

Status: draft

Purpose: define minimal language metadata fields in the `ma` extension namespace of a `did:ma` document.

## 1. Scope

This specification defines actor language metadata under `ma.fields`.

It is intentionally minimal and only standardizes two fields:

- `language`: language preference chain
- `lang`: active language choice

This version does not define translation bundle discovery, remote locale negotiation, or fallback network behavior.

## 2. Location in DID Document

Language metadata MUST be stored in the `ma` extension namespace of the DID document, under `ma.fields`.

Logical shape:

```json
{
  "ma": {
    "fields": {
      "language": "nb_NO:en_UK:en",
      "lang": "nb_NO"
    }
  }
}
```

If `ma` is missing, implementations MAY create it.

If `ma.fields` is missing, implementations MAY create it.

If `ma` or `ma.fields` exists but is not an object/map, implementations MUST fail validation and MUST NOT mutate the document.

## 3. Field Semantics

### 3.1 `language`

`language` is the preference chain as a colon-separated list in GNU-style locale tokens.

Example:

```text
nb_NO:en_UK:en:da:sv
```

Interpretation order is left-to-right. Earlier entries have higher priority.

### 3.2 `lang`

`lang` is the currently active language in GNU-style locale token form.

Example:

```text
en_UK
```

## 4. Token Format (GNU-style)

Each language token MUST be one of:

- `ll` (two lowercase ASCII letters), for example `en`
- `ll_CC` (language + region), where `CC` is two uppercase ASCII letters, for example `nb_NO`

Hyphenated input forms such as `nb-NO` MAY be accepted by implementations, but normalized output in DID documents MUST use underscore form (`nb_NO`).

Invalid examples for this version:

- `nb_no` (region not uppercase)
- `eng` (invalid language length)
- `en_UK_POSIX` (extra segments not standardized in 0.0.1)

## 5. Validation Rules

An implementation handling these fields MUST reject updates unless all rules are met:

1. `ma` is absent or an object/map.
2. `ma.fields` is absent or an object/map.
3. `language` is a non-empty colon-separated list of valid tokens.
4. `lang` is a single valid token.
5. If `lang` is not present in `language`, this is allowed in 0.0.1 (local override).

## 6. Update Semantics

When these fields are changed in a DID document:

- The document MUST be re-signed.
- The updated document MUST pass signature verification.
- The updated document SHOULD be re-published via `/ma/ipfs/0.0.1` (or equivalent home flow).

## 7. Interoperability Profile

For interoperability in this repository, a conforming implementation MUST:

- support read/write of `ma.fields.language`
- support read/write of `ma.fields.lang`
- enforce GNU-style token validation above
- preserve the fields during JSON and DAG-CBOR round-trips

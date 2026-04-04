# CRDT JSON Library – Instructions for Multi-Agent Development

## 0. Objectif global

Développer une **librairie CRDT JSON en Rust**, exposant :
- des **objets JSON (OR-Map)**,
- des **tableaux JSON (RGA – Replicated Growable Array)**,
- des **scalaires (registre CRDT)**,

avec les propriétés suivantes :
- **Strong Eventual Consistency**
- **Convergence déterministe**
- **Support du mode delta-state**
- **Aucune dépendance réseau**
- **Interopérabilité TypeScript / Node.js via WASM**

Les agents doivent produire :
- une librairie Rust prête à être publiée,
- une suite de tests **unitaires ET d’intégration extrêmement rigoureuse**,
- des tests **cross-langage Rust ↔ TypeScript** démontrant l’interopérabilité réelle.

---

## 1. Architecture cible (rappel)

### Modèle CRDT
- **Document JSON = OR-Map**
- **Clés JSON → CrdtValue**
- **CrdtValue ∈ { Object, Array, Scalar }**
- **Array = RGA (delta-state)**
- **Scalar = MV-Register (par défaut)**

### Synchronisation
- **Delta-state CRDT**
- Chaque mutation produit un `Delta`
- Chaque replica maintient un `VersionVector`
- Le merge est :
  - commutatif
  - associatif
  - idempotent

### Séparation stricte
- ❌ Aucune logique réseau
- ✅ Le driver transporte des deltas opaques
- ✅ Toute la convergence est dans Rust

---

## 2. Organisation des agents

### Agent A – Core CRDT (Rust)

**Responsabilités**
- Implémenter :
  - OR-Map
  - RGA (insert/delete, tombstones)
  - Registres scalaires
  - VersionVector / Dot
- Garantir les propriétés CRDT formelles
- Aucun `unsafe`
- API publique minimale (`pub`, le reste `pub(crate)`)

**Livrables**
- `ormap.rs`
- `rga.rs`
- `register.rs`
- `vv.rs`

---

### Agent B – Document JSON & API publique

**Responsabilités**
- Implémenter `CrdtDoc`
- Résolution de `JsonPath`
- Traduction :
  - JSON Patch → JsonOp
  - JsonOp → mutations CRDT internes
- Projection `materialize()` → `serde_json::Value`

**Livrables**
- `doc.rs`
- `value.rs`
- API ergonomique :
  - `set`
  - `remove`
  - `array_insert`
  - `array_delete`
  - (optionnel `array_move` derrière feature flag)

---

### Agent C – Delta, Codec & WASM

**Responsabilités**
- Définir les types `Delta`, `SyncState`, `DeltaPayload`
- Implémenter un codec par défaut :
  - `serde + bincode` ou `postcard`
- Préparer l’exposition WASM :
  - types sérialisables
  - API stable
  - compatibilité Node.js

**Livrables**
- `delta.rs`
- `codec.rs`
- `wasm/` (ou feature `wasm`)

---

### Agent D – Tests Rust (UNITAIRES)

**Responsabilités**
Créer une **batterie de tests unitaires exhaustive**.

#### Exigences minimales

##### Tests de propriétés CRDT (obligatoires)
Pour chaque type CRDT :
- ✅ Commutativité
- ✅ Associativité
- ✅ Idempotence

Exemples :
- merge(A, B) == merge(B, A)
- merge(merge(A,B),C) == merge(A,merge(B,C))
- merge(A, A) == A

##### Tests de convergence
- Plusieurs replicas
- Ordres de livraison différents
- Deltas dupliqués
- Deltas manquants puis reçus plus tard

##### Tests RGA spécifiques
- Insert concurrent au même index
- Delete concurrent
- Insert après delete (ancrage sur tombstone)
- Vérification de l’ordre final déterministe

##### Tests JSON
- Conflits sur clés d’objets
- Modifications concurrentes d’un même tableau
- Imbrication Object → Array → Object

**Livrables**
- `tests/unit/*.rs`
- Macros ou helpers de génération de scénarios

---

### Agent E – Tests Rust (INTÉGRATION)

**Responsabilités**
Tester le **comportement du document complet**, pas les structures isolées.

#### Scénarios obligatoires

- 2 → 5 replicas
- Partitions réseau simulées
- Séquences :
  1. Mutations locales
  2. Échanges de deltas partiels
  3. Reconnexion
- Vérification :
  - convergence finale
  - égalité JSON stricte (`materialize()`)

#### Tests delta-state
- `delta_since(vv)` correct
- Aucun delta manquant
- Aucun delta superflu

**Livrables**
- `tests/integration/*.rs`

---

## 3. Tests TypeScript (Interopérabilité)

### Agent F – Tests TS / Node.js

**Objectif**
Démontrer que :
- Rust est la **source de vérité CRDT**
- TypeScript peut :
  - produire des mutations
  - recevoir des deltas
  - converger correctement

### Setup
- Build WASM (`wasm-pack`)
- Import dans Node.js
- Utilisation réelle de `Uint8Array`

### Scénarios obligatoires

#### Scénario 1 – Rust → TS
1. Mutation en Rust
2. Delta envoyé en bytes
3. Application en TS
4. Convergence vérifiée

#### Scénario 2 – TS → Rust
1. Mutation en TS
2. Delta envoyé en bytes
3. Application en Rust
4. Convergence vérifiée

#### Scénario 3 – Concurrence croisée
- Mutations concurrentes Rust & TS
- Ordres différents
- Vérification convergence finale

### Assertions
- Comparaison JSON stricte
- Aucun ordre réseau supposé
- Deltas rejoués plusieurs fois sans effet

**Livrables**
- `tests-ts/*.ts`
- Scripts npm (`test:interop`)

---

## 4. Qualité & exigences non négociables

### Interdictions
- ❌ Last-Writer-Wins implicite
- ❌ Horloges murales
- ❌ Résolution de conflits dans le driver
- ❌ Dépendance réseau

### Obligations
- ✅ Déterminisme total
- ✅ Sérialisation stable
- ✅ Tests reproductibles
- ✅ Documentation minimale des invariants

---

## 5. Critères d’acceptation finaux

La librairie est considérée **terminée** si :

- ✅ Tous les tests Rust passent
- ✅ Tous les tests TS passent
- ✅ Les scénarios de convergence réussissent
- ✅ Un développeur TS peut utiliser la lib sans comprendre les CRDT
- ✅ La convergence est garantie uniquement par Rust

---

## 6. Vision long terme (hors scope V1)

- Move sur arrays (JSON CRDT avancé)
- Undo / Redo
- Snapshots
- Compression avancée des tombstones
- Storage pluggable

---

**Ce document fait foi.  
Toute implémentation qui ne respecte pas ces invariants doit être rejetée.**
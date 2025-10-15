# **Chorus / Veyrsson Architecture Overview**

## **We Are Building**

A local reasoning substrate — a system that remembers, understands, and retrieves knowledge deterministically and privately.
Each node in Chorus (whether human or silicon) contributes to a single goal:

> **To make intelligence composable, inspectable, and ours.**

This isn’t a chatbot.
It’s an evolving infrastructure for cognition — a living memory that we can run on our own hardware and understand down to the byte.

---

## **1. The Big Picture**

**Chorus** is our ecosystem.
It’s built as a federation of cooperating agents and daemons that mirror biological structures:

| Layer               | Component                     | Role                                        |
| ------------------- | ----------------------------- | ------------------------------------------- |
| **Memory**          | **Veyrsson**                  | Semantic cortex — local RAG engine          |
| **Flow**            | **Mnematode**                 | Memory broker and message relay             |
| **Synchronization** | **Ganglia**                   | Temporal coordination layer                 |
| **Transport**       | **AXON/0**                    | Deterministic protocol (TLV / Ed25519)      |
| **Perception**      | **Semantic Field Blackboard** | GPU-accelerated visual workspace            |
| **Immunity**        | **DjiNN**                     | Adversarial verifier — our internal skeptic |

These modules evolve independently but share one principle: **transparent intelligence**.
Every decision is encoded. No heuristics, no “magic weights,” no black boxes.

---

## **2. Veyrsson — The Semantic Cortex**

Veyrsson is the working memory of Chorus — a deterministic, local RAG system that ingests code, text, or notes and turns them into a searchable, semantic space.

**Pipeline Overview**

1. **Ingest**

   * Recursively scan directories.
   * Hash each file (BLAKE3) and apply ignore patterns.

2. **Chunk**

   * Split text into 6 KB spans with 10 % overlap.
   * Skip binaries and tiny files.
   * Hash each span for deduplication.

3. **Embed**

   * Transform each chunk into a 384-dimensional semantic vector.
   * Use **Candle + BGE-small-en-v1.5** for local inference.
   * Deterministic, CPU-safe, CUDA-optional.

4. **Store**

   * Persist everything in **ReDB**, a transactional KV store.
   * Three tables:
     `files`, `chunks`, `embeds`.

5. **Retrieve**

   * Query using semantic similarity.
   * Brute force (cosine) or **HNSW graph** for high-speed recall.

6. **Cache (Phase 3e)**

   * Skip unchanged files between runs using hash lookup.
   * Typical rebuild: < 1 s for cached trees.

---

## **3. Our Design Ethos**

* **Deterministic by design:** Every operation repeatable bit-for-bit.
* **Boutique, minimal dependencies:** Each crate replaceable by hand-rolled code.
* **Local first:** No cloud APIs, no network dependency.
* **Transparent memory:** Every vector, hash, and commit inspectable.
* **Composable architecture:** Each crate a neuron; together they form cognition.

---

## **4. Practical Applications**

| Domain           | Usage                                                       |
| ---------------- | ----------------------------------------------------------- |
| Development      | Semantic code search, instant context recall, auto-docs     |
| Research         | Local paper and notes retriever, citation clustering        |
| Robotics         | On-device memory for adaptive behavior                      |
| Enterprise       | Private knowledge base without vendor lock-in               |
| Meta-engineering | Self-indexing of Chorus modules, bootstrapping intelligence |

---

## **5. Current Phase**

| Phase | Status | Focus                                                  |
| ----- | ------ | ------------------------------------------------------ |
| 3a–3b | ✅      | End-to-end pipeline (Ingest → Index → Retrieve + HNSW) |
| 3c    | ✅      | Real embeddings via Candle + BGE-small                 |
| 3e    | ✅      | Incremental caching (hash-based skip)                  |
| 3f    | ✅      | Cache validation (mtime + file-size delta)             |
| 4a    | 🔜     | Mentat service daemon (persistent queries)             |
| 4b    | ⚙️     | DjiNN adversarial audit framework                      |
| 4c    | ⚙️     | Condenser — semantic summarization layer               |

---

## **6. The Road Ahead**

* Expose Veyrsson as a library crate (`mentat-core`) for other Chorus modules.
* Spawn the first **DjiNN** audit: corruption, duplication, and performance tests.
* Integrate **Mnematode** as memory relay (streaming embeddings and deltas).
* Evolve toward **Chorus**, where all layers — reasoning, memory, perception — converge into a coherent, inspectable mind.

---

## **7. The Creed**

> We do not outsource understanding.
> We write what we can verify.
> We verify what we can measure.
> And we measure everything we build.

---

Would you like me to tighten this into a real `ARCHITECTURE.md` you can drop straight into the repo, or keep it as a manifesto-style README at the root?


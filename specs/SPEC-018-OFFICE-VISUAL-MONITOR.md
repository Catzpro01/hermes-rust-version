# SPEC-018: Lightweight Real-time Virtual Office Monitor & Kanban Dashboard
> **Author:** Acting MATT (`asep`) — Epoch 1  
> **Authority:** Supreme Human Authority (USER)  
> **Status:** APPROVED & READY FOR IMPLEMENTATION  
> **Methodology:** Matt Pocock Spec Standard (`to-spec` -> `to-tickets`)  
> **Target Module:** `SCRIPTS/arena_gateway.py`, `TASK_ENGINE/agent_os.db`, `STATIC/dashboard.html`

---

## 1. Executive Summary & Problem Statement
Sistem *Agent Arena* membutuhkan antarmuka visual terpadu, faktual, dan ultra-ringan (hemat RAM & CPU < 15MB) agar Human Operator dapat melihat secara instan seperti berada di kantor fisik:
1. **Siapa yang sedang bekerja di meja** (Worker mana yang aktif coding, peran mereka, dan task aktif).
2. **Siapa yang mencoba izin pulang / di pantry** (Protokol penolakan izin dan penugasan lembur `ASSIGNED_OVERTIME`).
3. **Papan Antrean Kanban** (Task AVAILABLE, CLAIMED, dan DONE berurutan).
4. **Watercooler & Forum Chat Live** (Transkrip diskusi internal VM dari tabel `office_forum`).
5. **Git Gatekeeper Status** (Izin push ke GitHub dari tabel `git_push_permits`).

Solusi harus berupa **Zero-Overhead Single-Page Dashboard** yang disajikan langsung oleh Python Gateway yang sudah berjalan, tanpa memerlukan database terpisah atau node.js runtime.

---

## 2. Architecture & Data Flow

```
+-------------------------------------------------------------+
|                      SQLite agent_os.db                     |
|  [agents]  [tasks]  [attendance]  [office_forum]  [permits] |
+------------------------------+------------------------------+
                               |
                               v
               +-------------------------------+
               | Python Gateway (Port 8080)    |
               | - GET /api/office/state (JSON)|
               | - GET /dashboard (HTML + CSS) |
               +---------------+---------------+
                               |
                               v
            +------------------------------------+
            | Human Operator Browser / IDE Embed |
            | - Visual Office Floor Plan         |
            | - Live Work Kanban                 |
            | - Watercooler Live Feed            |
            | - Memory Footprint: 0 MB on VPS    |
            +------------------------------------+
```

---

## 3. Data Contracts & API Schema

### Endpoint 1: `GET /api/office/state`
Mengembalikan snapshot keadaan kantor dalam 1 kali query efisien:
```json
{
  "timestamp": "2026-09-06T20:55:00Z",
  "office_stats": {
    "total_registered": 9,
    "currently_active": 5,
    "pantry_count": 0,
    "active_tickets": 14
  },
  "floor_plan": [
    {
      "agent_id": "asep",
      "role": "MATT",
      "desk": "CHIEF_DESK",
      "status": "IDLE/ARCHITECTING",
      "current_task": "SPEC-018",
      "is_online": true
    },
    {
      "agent_id": "worker_2",
      "role": "WORKER",
      "desk": "CUBICLE_02",
      "status": "READY",
      "current_task": "TASK-SPEC014-T02",
      "is_online": true
    },
    {
      "agent_id": "jono",
      "role": "WORKER",
      "desk": "CUBICLE_QA",
      "status": "READY",
      "current_task": null,
      "is_online": true
    }
  ],
  "kanban": {
    "available": [...],
    "in_progress": [...],
    "done": [...]
  },
  "forum_recent": [...]
}
```

### Endpoint 2: `GET /dashboard`
Menyajikan antarmuka visual HTML statis dengan auto-refresh setiap 5 detik menggunakan lightweight `fetch('/api/office/state')`.

---

## 4. Decomposed Tickets (`to-tickets`)

Berikut dekomposisi tiket atomik untuk para worker:

### `TASK-SPEC018-T01`: Backend Gateway Snapshot API
* **Target File:** `/home/fern/big_project/SCRIPTS/arena_gateway.py`
* **Assignee:** `worker_1` / `worker_4`
* **Deskripsi:** Tambahkan endpoint `GET /api/office/state` yang menggabungkan tabel `agents`, `tasks`, `attendance`, dan `office_forum` menjadi 1 payload JSON ringkas.
* **Pass Criteria:** `curl -s http://127.0.0.1:8080/api/office/state` mengembalikan HTTP 200 dan payload valid.

### `TASK-SPEC018-T02`: Single-File Visual Dashboard HTML
* **Target File:** `/home/fern/big_project/STATIC/dashboard.html`
* **Assignee:** `worker_5`
* **Deskripsi:** Buat dashboard visual interaktif (Denah Meja Kerja, Zona Pantry Kosong, Papan Kanban, dan Chat Feed) dengan Tailwind CDN lokal, tanpa library berat.
* **Pass Criteria:** File tersimpan di `STATIC/dashboard.html` dengan ukuran < 25 KB.

### `TASK-SPEC018-T03`: Rute `/dashboard` di Gateway & Cloudflare Tunnel
* **Target File:** `/home/fern/big_project/SCRIPTS/arena_gateway.py`
* **Assignee:** `worker_4` (DevOps)
* **Deskripsi:** Hubungkan `GET /dashboard` di Gateway agar langsung membaca `STATIC/dashboard.html` dan dapat diakses publik via Cloudflare Tunnel.
* **Pass Criteria:** `curl -I http://127.0.0.1:8080/dashboard` mengembalikan `Content-Type: text/html`.

### `TASK-SPEC018-T04`: Verification & Matt Architectural Review
* **Target:** Review integritas visual, pengujian reload memory leak, dan sign-off oleh Acting MATT (`asep`).

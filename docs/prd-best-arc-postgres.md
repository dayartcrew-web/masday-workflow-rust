Untuk membangun arsitektur aplikasi Node.js yang kokoh, terstruktur, dan anti-stale connection (koneksi database mati/gantung) layaknya Laravel atau CodeIgniter, Anda membutuhkan kombinasi Framework MVC/Enterprise dan Koneksi Pooling yang Agresif.
Masalah stale connection (koneksi terputus tiba-tiba karena timeout jaringan atau idle di server database) sangat sering terjadi di Node.js karena sifatnya yang long-running process (berbeda dengan PHP yang mati-hidup setiap ada request).
Berikut adalah cetak biru (blueprint) arsitektur terbaik untuk mengatasinya.
------------------------------
## 1. Pilihan Framework (Kembaran Laravel di Node.js)
Jangan gunakan Express biasa jika Anda ingin pengalaman seperti Laravel. Gunakan framework dengan arsitektur terstruktur:

* NestJS (Sangat Direkomendasikan): Menggunakan TypeScript, memiliki arsitektur berbasis Dependency Injection (sangat mirip Laravel Service Container), dan memiliki modul database bawaan yang matang.
* AdonisJS: Ini adalah "Laravel-nya Node.js". Struktur folder, sistem routing, ORM (Lucid), hingga command line interface (Ace) dibuat meniru persis ekosistem Laravel.

------------------------------
## 2. Strategi Anti-Stale Connection (Level Database)
Kunci utama agar koneksi tidak pernah stale (basi) adalah dengan mengonfigurasi Connection Pool secara agresif menggunakan fitur Health Check / Keep-Alive.
Jika Anda menggunakan ORM standar industri seperti Prisma, Drizzle, atau TypeORM, mereka menggunakan driver pg di latar belakang. Konfigurasikan parameter internal pool berikut:
## Opsi A: Konfigurasi via Connection String (Paling Mudah)
Tambahkan parameter TCP Keep-Alive langsung pada URI database Anda di file .env:

DATABASE_URL="postgresql://user:password@localhost:5432/mydb?keepAlive=true&connection_timeout=10&idle_timeout=20&max=20"

## Opsi B: Konfigurasi Programatis (Jika Menggunakan pg Pool)
Jika Anda membangun koneksi manual di layer repositori/model, gunakan konfigurasi ini:

const { Pool } = require('pg');
const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  max: 20,                  // Maksimal koneksi dalam pool
  idleTimeoutMillis: 30000, // Tutup koneksi otomatis jika idle 30 detik (mencegah stale)
  connectionTimeoutMillis: 2000, // Beri error jika gagal connect dalam 2 detik
  
  // KUNCI UTAMA ANTI-STALE: Aktifkan TCP KeepAlive level OS
  keepalive: true,
  keepaliveInitialDelayMillis: 10000 // Kirim paket ping setiap 10 detik ke Postgres
});
// Error handling global untuk menangkap koneksi yang mati di latar belakang
pool.on('error', (err, client) => {
  console.error('Koneksi pool drop tiba-tiba:', err.message);
  // Node-postgres otomatis akan menghapus client yang rusak ini dari pool
});

------------------------------
## 3. Pola Arsitektur Folder (Struktur MVC / Clean Architecture)
Agar kode Anda serapi Laravel, terapkan struktur Layered Architecture (Controller -> Service -> Repository/Model). Berikut adalah contoh struktur direktori pada aplikasi Node.js/TypeScript:

src/
├── config/
│   └── database.ts       # Konfigurasi pool & pencegahan stale connection
├── controllers/
│   └── UserController.ts # Validasi request & panggil service (seperti Laravel Controller)
├── services/
│   └── UserService.ts    # Tempat Logic Bisnis Utama (seperti Laravel Service/Job)
├── repositories/
│   └── UserRepository.ts # Query mentah SQL / pemanggilan ORM
├── models/
│   └── User.ts           # Definisi skema / tipe data entitas
└── app.ts                # Entry point aplikasi

## Contoh Implementasi Layer Repositori yang Aman dari Stale
Koneksi database disuntikkan (injected) ke dalam kelas, dan setiap query dipastikan mengambil koneksi yang valid dari pool.

// src/repositories/UserRepository.tsimport { pool } from '../config/database';
export class UserRepository {
  async findActiveUsers() {
    // pool.query otomatis mengambil koneksi sehat dari pool, atau membuat yang baru jika stale
    const { rows } = await pool.query('SELECT * FROM users WHERE is_active = true');
    return rows;
  }
}

------------------------------
## Summary Checklist Anti-Stale:

   1. Gunakan Pool, Jangan Single Client: Jangan pernah menggunakan new Client() dari pg untuk aplikasi web global. Selalu gunakan new Pool().
   2. Aktifkan keepalive: true: Ini memastikan server Node.js Anda rajin "menyapa" PostgreSQL agar firewall atau router tidak memutus koneksi sepihak karena dianggap pasif.
   3. Handle Pool Error: Selalu pasang fungsi pool.on('error') agar aplikasi tidak crash total saat database melakukan restart berkala.

Apakah Anda ingin melihat contoh setup lengkap menggunakan AdonisJS yang mirip Laravel, atau membutuhkan bantuan untuk migrasi skema database?


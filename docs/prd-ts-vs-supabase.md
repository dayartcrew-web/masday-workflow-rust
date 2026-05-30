Untuk kasus **stale peer connection** pada project TypeScript + Supabase, arsitektur yang aman adalah: pisahkan client berdasarkan runtime, pakai **singleton** hanya di browser/app runtime yang sama, tambahkan health-check + reconnect untuk Realtime, dan jangan menyimpan server client yang membawa auth user dalam singleton global lintas request. Supabase sendiri menjelaskan bahwa silent disconnect pada Realtime bisa terjadi saat heartbeat terganggu, terutama ketika app/background tab kena throttling browser, dan merekomendasikan `worker: true` plus `heartbeatCallback` untuk reconnect eksplisit. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

## Pola arsitektur

Buat 3 lapisan: `core/supabase`, `services`, dan `features`; `core/supabase` hanya menangani inisialisasi client, koneksi, retry, serta observability, sedangkan `services` membungkus query/domain logic agar komponen tidak memanggil Supabase langsung. Struktur ini membantu karena masalah stale connection biasanya bukan di query bisnisnya, tetapi di lifecycle client, token, dan websocket yang tersebar ke banyak file. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

Untuk TypeScript, pola yang umum dipakai:
- `browser client`: singleton per tab/app runtime.
- `server client`: factory per request, bukan singleton auth global.
- `admin/service-role client`: singleton hanya di backend process yang trusted dan stabil.
- `realtime manager`: module terpisah yang mengelola subscribe, reconnect, resubscribe. [reddit](https://www.reddit.com/r/Supabase/comments/1dkrasj/can_the_supabase_ssr_serverclient_be_in_a/)

## Aturan connection lifecycle

Untuk browser/frontend, hindari `createClient()` di banyak komponen karena itu bisa memunculkan banyak instance dan koneksi yang sulit dilacak; lebih baik satu instance dibagikan lewat module singleton atau dependency injection container per app runtime. Pendekatan ini konsisten dengan praktik community yang menekankan reuse client agar tidak memicu instance berlebih. [linkedin](https://www.linkedin.com/posts/arnold-musandu-3489b2293_stop-spinning-up-silent-supabase-instances-activity-7342475652001898496-9VXE)

Untuk SSR atau API handler, jangan jadikan client user-auth sebagai singleton global karena kredensial bisa tercampur antar request asynchronous; pola yang lebih aman adalah factory `createServerSupabaseClient(requestContext)` setiap request. Ada diskusi community yang menegaskan singleton untuk SSR server client berisiko membagi credential lintas koneksi. [reddit](https://www.reddit.com/r/Supabase/comments/1dkrasj/can_the_supabase_ssr_serverclient_be_in_a/)

## Solusi stale peer connection

Kalau yang stale adalah **Realtime/WebSocket**, aktifkan `worker: true` agar heartbeat tetap jalan di background, lalu pasang `heartbeatCallback` untuk mendeteksi status `disconnected` dan memanggil reconnect. Supabase merekomendasikan kombinasi dua opsi ini sebagai pendekatan paling robust untuk silent disconnection. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

Contoh bentuk modulnya:

```ts
// core/supabase/browser.ts
import { createClient, type SupabaseClient } from '@supabase/supabase-js'

let client: SupabaseClient | null = null

export function getBrowserSupabase() {
  if (client) return client

  client = createClient(
    import.meta.env.VITE_SUPABASE_URL,
    import.meta.env.VITE_SUPABASE_ANON_KEY,
    {
      realtime: {
        worker: true,
        heartbeatCallback: (status) => {
          if (status === 'disconnected') {
            client?.realtime.disconnect()
            client?.realtime.connect()
          }
        },
      },
    }
  )

  return client
}
```

Pattern di atas cocok bila masalahnya channel terlihat hidup tetapi sebenarnya websocket sudah tidak sehat, karena manager memusatkan reconnect logic di satu tempat. Dokumentasi Supabase menyebut silent disconnect sering terjadi saat browser men-throttle timer background dan heartbeat gagal terkirim. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

## Rekomendasi struktur folder

```txt
src/
  core/
    supabase/
      browser.ts
      server.ts
      admin.ts
      realtime-manager.ts
      connection-state.ts
  services/
    auth.service.ts
    profile.service.ts
    orders.service.ts
  features/
    chat/
      chat.repo.ts
      chat.realtime.ts
    trading/
      positions.repo.ts
      ticks.realtime.ts
  shared/
    types/
    utils/
```

Pisahkan `repo/service` dari `realtime subscription` karena query biasa dan websocket punya lifecycle berbeda; ini membuat stale peer issue tidak “bocor” ke seluruh codebase. Saat koneksi putus, cukup reset `realtime-manager` dan lakukan resubscribe dari feature yang terdampak. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

## Guardrail penting

- Tambahkan state koneksi seperti `connecting | healthy | stale | reconnecting | failed` agar UI tahu kapan harus fallback ke refetch biasa. Ide ini sejalan dengan pendekatan robust connection handling yang menekankan deteksi eksplisit, bukan asumsi koneksi masih sehat.  [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)
- Untuk channel penting, simpan metadata subscription dan buat mekanisme `resubscribeAll()` setelah reconnect. Ini perlu karena reconnect socket belum tentu otomatis mengembalikan seluruh state domain subscription dengan aman. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)
- Jika error yang muncul `TIMED_OUT` di Node runtime, cek versi Node; Supabase mendokumentasikan bahwa kombinasi `supabase-js` baru dengan Node lebih lama dari v22 bisa memicu masalah kompatibilitas Realtime. [supabase](https://supabase.com/docs/guides/troubleshooting/realtime-handling-silent-disconnections-in-backgrounded-applications-592794)
- Untuk akses database backend langsung dengan pool yang menumpuk idle connection, pertimbangkan pola client yang lebih tepat per workload; pada beberapa kasus, community menyarankan memakai client langsung alih-alih pool bila koneksi pendek dan sederhana. [stackoverflow](https://stackoverflow.com/questions/79156717/too-many-idle-database-connections-in-supabase)

Kalau mau, saya bisa lanjutkan dengan **template arsitektur TS siap pakai** untuk Vite/Next.js/NestJS yang khusus menangani stale Supabase connection beserta `ConnectionManager`, `RealtimeRegistry`, dan `health check` pattern.

Berikut **lanjutan yang paling relevan** untuk `next`: fokus ke arsitektur **Next.js + TypeScript + Supabase** supaya stale peer connection tidak muncul berulang. Supabase docs terbaru menekankan dua sumber masalah utama: silent Realtime disconnect di background tab dan `TIMED_OUT` yang bisa muncul karena ketidakcocokan `realtime-js`/Node versi lama, jadi solusi arsitekturnya harus menangani lifecycle client dan runtime compatibility sekaligus. [supabase](https://supabase.com/docs/guides/troubleshooting/realtime-connections-timed_out-status)

## Struktur Next.js

Pola yang saya sarankan:
- `src/lib/supabase/browser.ts` untuk client browser singleton.
- `src/lib/supabase/server.ts` untuk server client per request.
- `src/lib/supabase/admin.ts` untuk service-role client di backend trusted.
- `src/lib/supabase/realtime.ts` untuk subscribe, heartbeat, reconnect, dan resubscribe.
- `src/services/*` untuk query/business logic.
- `src/app/*` hanya memanggil services, bukan Supabase langsung. [supabase](https://supabase.com/docs/reference/javascript/typescript-support)

Di Next.js, jangan jadikan server client auth sebagai singleton global; untuk SSR dan route handler, buat client baru per request agar session tidak bocor antar user. Untuk browser, singleton aman selama hanya satu runtime/tab, dan itu memang lebih baik untuk mencegah client berlipat-lipat. [reddit](https://www.reddit.com/r/Supabase/comments/1dkrasj/can_the_supabase_ssr_serverclient_be_in_a/)

## Contoh implementasi

```ts
// src/lib/supabase/browser.ts
import { createClient } from '@supabase/supabase-js'
import type { Database } from '@/types/database'

let client: ReturnType<typeof createClient<Database>> | null = null

export function getBrowserSupabase() {
  if (client) return client

  client = createClient<Database>(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!,
    {
      realtime: {
        worker: true,
        heartbeatCallback: (status) => {
          if (status === 'disconnected') {
            client?.realtime.disconnect()
            client?.realtime.connect()
          }
        },
      },
    }
  )

  return client
}
```

```ts
// src/lib/supabase/server.ts
import { createServerClient } from '@supabase/ssr'
import { cookies } from 'next/headers'

export function createSupabaseServer() {
  const cookieStore = cookies()
  return createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!,
    {
      cookies: {
        get(name) {
          return cookieStore.get(name)?.value
        },
        set() {},
        remove() {},
      },
    }
  )
}
```

Model seperti ini memisahkan lifecycle browser dan server, sehingga stale peer issue di websocket tidak merusak query server-side. Supabase juga secara eksplisit mendorong pengelolaan koneksi dan pemantauan koneksi aktif untuk mencegah leak dan idle connection yang menumpuk. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

## Realtime manager

Untuk channel penting, buat registry kecil agar resubscribe bisa dilakukan terpusat:
- simpan daftar channel aktif,
- detach listener lama sebelum reconnect,
- lakukan `resubscribeAll()` setelah reconnect,
- fallback ke refetch polling bila status koneksi `stale` terlalu lama. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

Ini penting karena reconnect websocket tidak selalu mengembalikan state fitur secara otomatis; kalau app trading, chat, atau dashboard live, kamu perlu “source of truth” yang bisa dipulihkan, bukan hanya socket yang reconnect. Supabase docs tentang silent disconnection menekankan reconnect eksplisit dan heartbeat awareness, bukan asumsi auto-heal penuh. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)

## Supabase versioning

Kalau kamu melihat error aneh setelah upgrade, pastikan versi `@supabase/supabase-js` dan generated types kompatibel. Ada issue terbaru yang menunjukkan v2.50.4 sempat mematahkan kompatibilitas type dengan generated `Database` types, dan v2.50.5 mengembalikan kompatibilitas tersebut. [github](https://github.com/supabase/supabase-js/issues/1483)

Artinya, untuk project production, pin versi `supabase-js`, generate types lewat CLI, lalu upgrade dengan changelog review, bukan caret bebas. Dokumentasi TypeScript Supabase juga memang menganjurkan generated database types dipakai langsung saat `createClient<Database>()`. [supabase](https://supabase.com/docs/reference/javascript/typescript-support)

## Checklist cepat

- Pakai singleton hanya di browser runtime. [linkedin](https://www.linkedin.com/posts/arnold-musandu-3489b2293_stop-spinning-up-silent-supabase-instances-activity-7342475652001898496-9VXE)
- Pakai factory per request untuk SSR/server route. [reddit](https://www.reddit.com/r/Supabase/comments/1dkrasj/can_the_supabase_ssr_serverclient_be_in_a/)
- Aktifkan `worker: true` untuk Realtime background handling. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)
- Tambahkan `heartbeatCallback` untuk reconnect eksplisit. [reddit](https://www.reddit.com/r/Supabase/comments/1pkvs7i/why_do_i_need_to_refresh_my_supabase_connection/)
- Upgrade Node ke versi terbaru LTS jika kena `TIMED_OUT`. [supabase](https://supabase.com/docs/guides/troubleshooting/realtime-connections-timed_out-status)
- Pin `@supabase/supabase-js` dan generated types. [github](https://github.com/supabase/supabase-js/issues/1483)

Saya bisa lanjutkan dengan **template folder + code lengkap Next.js App Router** untuk kasus ini, termasuk `realtime-manager.ts`, `useSupabaseRealtime` hook, dan `ConnectionState` store.
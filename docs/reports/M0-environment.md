# M0 — Environment report (dev laptop)

Blueprint reference: section 2.19 step 1. Measured on **21 November 2026**.
Raw data: [`dxdiag.txt`](dxdiag.txt) (same folder).

This report resolves open question **Q3** (exact laptop specs) and makes decision **A5** concrete.

---

## 1. Verdict

| | |
| --- | --- |
| **Preset tier** | **Minimum / Low** (blueprint 1.5) |
| **Realistic target** | 1080p @ 30 fps, active city capped to the central **8 km × 8 km**, Tier 2 agent cap **5,000** |
| **Panel** | 1920 × 1200 @ 60 Hz — **no 1440p**, so the 1440p row of any FPS table is not testable on this machine |
| **Memory budget that applies** | `Low under 3 GB total` (2.13) — the iGPU shares system RAM, so there is no separate VRAM pool |

The machine is a thin-and-light office laptop with a **low-power integrated GPU**. It is at or
slightly below the blueprint's "Intel Iris Xe class" reference for the Minimum tier. Every milestone
must therefore default the dev build to **Low** and assume no headroom.

---

## 2. Hardware

### System

| Item | Value |
| --- | --- |
| Manufacturer / model | LENOVO `21M8S52Q00` |
| OS | Windows 11 Enterprise 64-bit, 10.0 Build **26200** |
| DirectX | DirectX 12 (feature levels up to **12_2**) |
| Driver model | WDDM 3.2 |
| Graphics backend in use | **Vulkan** (measured at runtime, not DX12 as `dxdiag` suggested) |

### CPU

| Item | Value |
| --- | --- |
| Model | **Intel Core Ultra 5 125U** (Meteor Lake-U) |
| Cores / threads | **12 cores / 14 threads** (2 P-cores + 8 E-cores + 2 LP E-cores) |
| Base clock (reported) | ~1.3 GHz (boosts well above this; U-series = 15 W class TDP) |

**Implication for the simulation.** 14 threads is genuinely good for `rayon` — the sim-core budget of
"under 40 ms per tick at 1M citizens" (2.13) is plausible here *on CPU*. But P-core count is low and
this is a 15 W part, so sustained all-core throughput will be well under a desktop's. Benchmarks must
report sustained numbers, not a first-run burst.

### RAM

| Item | Value |
| --- | --- |
| Total physical | **15.48 GB** usable (16 GB installed) |
| Configuration | 1 × 16 GB Samsung @ 5600 MHz — **single channel** |
| Page file at measurement | 25,943 MB used / 5,755 MB available |

Two things matter here:

1. **Single channel.** The iGPU shares this one memory channel with the CPU. Memory bandwidth is the
   single biggest bottleneck for an integrated GPU, and a dual-channel config would roughly double
   it. This is the main reason to expect Low-tier performance despite 16 GB.
2. **Memory pressure is already high** at measurement time (large page-file usage). Keeping the game
   under the 3 GB Low budget is not optional.

### GPU

| Item | Value |
| --- | --- |
| Adapter | **Intel(R) Graphics** (integrated, Xe-LPG / Arc-generation graphics in Meteor Lake) |
| PCI IDs | Vendor `0x8086`, Device `0x7D45` |
| Dedicated memory | **128 MB** |
| Shared memory | **9,034 MB** (carved from system RAM) |
| Total display memory | 9,162 MB |
| Driver | **32.0.101.8332**, dated 2025-12-12 |
| Feature levels | 12_2, 12_1, 12_0, 11_1, 11_0, 10_1, 10_0, 9_3, 9_2, 9_1, 1_0_CORE |

An NPU (`0x7D1D`, Intel AI Boost, driver 32.0.100.4724) is also present. It is irrelevant to this
project and will not be used.

**Backend actually chosen: Vulkan.** DirectX 12 with feature level 12_2 is available, and I expected
`wgpu` to pick DX12. **It picked Vulkan instead.** Measured at runtime on 2026-11-21 by the app's own
startup logging (M0 step 4), which is the authoritative record:

```text
AdapterInfo {
    name: "Intel(R) Graphics",
    vendor: 32902,            // 0x8086 Intel
    device: 32069,            // 0x7D45 Meteor Lake-U iGPU
    device_type: IntegratedGpu,
    driver: "Intel Corporation",
    driver_info: "101.8332",
    backend: Vulkan,
    subgroup_min_size: 8,
    subgroup_max_size: 32,
}
```

This is exactly why blueprint 2.19 step 1 asks for the adapter to be printed at startup rather than
inferred from `dxdiag`: what Windows advertises and what `wgpu` selects are different questions.

Two capability lines from the same run are better news than expected for a Minimum-tier machine:

```text
INFO bevy_pbr::cluster: GPU clustering is supported on this device.
INFO bevy_render::batching::gpu_preprocessing: GPU preprocessing is fully supported on this device.
```

GPU preprocessing and clustered lighting both working means Bevy's GPU-driven rendering path
(blueprint 2.6.1) is available here, which matters for drawing 4,096 terrain chunks and, later, for
night-time city lights.

One harmless warning also appears every run and is worth recording so it is not mistaken for a fault
later:

```text
WARN wgpu_hal::vulkan::instance: windows_read_data_files_in_registry:
     Registry lookup failed to get layer manifest files.
```

That is the Vulkan loader finding no optional validation-layer manifests registered. It has no effect
on rendering.

### Display

| Item | Value |
| --- | --- |
| Panel | Lenovo "Wide viewing angle & High density FlexView Display 1920x1200" |
| Native mode | **1920 × 1200 @ 60.003 Hz** |
| Current mode | 1920 × 1200 (32 bit) @ 60 Hz |
| Windows scaling | 150% (desktop reports a 1280 × 800 logical work area) |

Consequences:

- **60 Hz cap.** FPS measurement MUST run with vsync **off**, or every preset will read exactly 60.
  M0 step 4 already includes a vsync toggle; the FPS table must state vsync off.
- **No 1440p.** The blueprint asks for a 1440p FPS row "if my screen supports it" — it does not. The
  M0 report will say so rather than invent numbers. 1440p can only be checked later on an external
  monitor.
- **150% scaling** means the window must be driven by physical pixels for honest benchmarking;
  logical-pixel sizing would silently render at 1280 × 800.

### Storage

| Drive | Free | Used |
| --- | --- | --- |
| `C:` (system) | **29.8 GB** | 143.3 GB |
| `D:` (project) | **214.9 GB** | 87.7 GB |

`D:` matches decision A5 (~200 GB free) and comfortably holds the repo plus Rust `target/`
directories, which grow to several GB per profile with Bevy.

> **`C:` is tight.** 29.8 GB free, and the toolchain installs land there (VS Build Tools, rustup,
> cargo registry under `%USERPROFILE%\.cargo`). This is a tracked risk — see section 5.

---

## 3. Toolchain found before any installation

| Tool | Status | Version |
| --- | --- | --- |
| rustup | ✅ present | 1.29.3 (2026-11-02) |
| rustc | ✅ present | **1.99.1** (2026-10-13) |
| cargo | ✅ present | 1.99.1 |
| git | ✅ present | 2.55.0.windows.3 |
| git-lfs | ✅ present | 3.7.1 |
| cmake | ✅ present | 4.4.4 |
| winget | ✅ present | v1.30.310 |
| node | ✅ present | v24.18.0 (not needed by this project) |
| Python | ✅ present | 3.11.15 and 3.14.6 |
| **Visual Studio Build Tools** | ❌ **MISSING** | no `link.exe`, no Windows SDK found |
| **MSVC default toolchain** | ⚠️ **WRONG** | default was `stable-x86_64-pc-windows-gnu`; msvc was installed but not active |
| clippy / rustfmt | ✅ present | already installed for both gnu and msvc |
| rust-src | ❌ MISSING | not installed for msvc |
| cargo-nextest | ❌ MISSING | — |
| cargo-deny | ❌ MISSING | — |
| Blender | ❌ MISSING | optional (2.2), not needed for M0 |

### The two real problems found

1. **No MSVC C++ toolchain at all.** No Visual Studio, no Build Tools, no Windows SDK. Only SQL
   Server Management Studio 22 was present. Without this, the `x86_64-pc-windows-msvc` target cannot
   link.
2. **rustup's default toolchain was the GNU one.** The MSVC toolchain was installed but not active,
   so every build would have used `windows-gnu` — diverging from what the blueprint (2.2, 2.15)
   requires and from what CI will build. Its `clippy` and `rustfmt` were already in place; only
   `rust-src` was missing.

Both were fixed during bootstrap — see [`docs/TOOLING.md`](../TOOLING.md) for what was installed and
why. After the fix, `rustc --version --verbose` reports `host: x86_64-pc-windows-msvc`, and a minimal
Rust binary was compiled, linked and **run** successfully against the MSVC target (toolset
14.44.35207, Windows SDK 10.0.26100.0). `C:` free space after the install: **24.7 GB**.

**Why MSVC and not GNU**, since GNU would have worked without a 5 GB install: the blueprint specifies
MSVC (2.2 "MSVC toolchain on Windows", 2.15 prerequisites), it is the better-supported target for
`wgpu`/DX12 and for Tracy profiling, and CI release builds target MSVC. Switching the dev machine to
GNU would be an architecture deviation needing an ADR, for a worse result.

---

## 4. Rust version compatibility with the pinned engine

| | |
| --- | --- |
| Bevy pinned | **0.20.1** (latest stable, published 2026-10-28) |
| Bevy 0.20.1 MSRV | 1.97.1 |
| Installed rustc | **1.99.1** ✅ |
| Edition required by blueprint | 2024 ✅ (supported since 1.85) |

No Rust upgrade is needed.

---

## 5. Risks specific to this machine

| Risk | Severity | Mitigation |
| --- | --- | --- |
| **Single-channel RAM shared with the iGPU** — memory bandwidth is the primary limit for both render and sim | **High** | Low preset by default; 75% render scale; keep total under 3 GB; prefer instancing and impostors early; measure, never assume |
| **`C:` only has 29.8 GB free** and all toolchains install there | **High** | VS Build Tools installed with the minimal VCTools workload; keep all `target/` output on `D:`; monitor free space before adding tools. If `C:` gets tight, move the cargo registry to `D:` via `CARGO_HOME` |
| **60 Hz panel** will silently cap every FPS reading at 60 | Medium | Benchmark with vsync off; state it in every FPS table |
| **No 1440p panel** — a blueprint success criterion (A8, 1.9) cannot be verified here | Medium | Report 1080p only; flag 1440p as unverified until an external monitor is available |
| **15 W sustained power limit** — performance drops after thermal saturation | Medium | Benchmarks must run long enough to reach steady state and report sustained, not peak, numbers |
| **16 km terrain at 4 m resolution is ~32 MB of heightfield** plus 4,096 render chunks on a weak iGPU | Medium | This is exactly what M0 step 5 must prove. Aggressive chunk LOD and streaming radii; the Low preset's 8 km cap exists for this machine |
| High existing page-file pressure suggests other software is already using a lot of RAM | Low | Close other apps before benchmarking; note what was running in the report |

---

## 6. Answers this report provides

- **Q3 (exact laptop specs)** — answered above. **Resolved.**
- **A5 (dev machine)** — concrete: Lenovo `21M8S52Q00`, Core Ultra 5 125U, 15.48 GB single-channel
  RAM, Intel Xe-LPG iGPU, 1920 × 1200 @ 60 Hz, 214.9 GB free on `D:`.
- **A7 (hardware target)** — the minimum spec the blueprint asked Claude to choose is now anchored to
  real hardware: **this laptop is the Low-preset reference machine.**

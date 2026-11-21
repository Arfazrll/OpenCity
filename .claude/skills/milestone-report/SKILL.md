---
name: milestone-report
description: Write the end-of-milestone report for the owner in Bahasa Indonesia. Use when a milestone's Definition of Done is met, or when the user asks for the milestone report or laporan milestone.
---

# milestone-report

1. **Gather**: plan checklist status (`docs/plans/<Mx>.md`), `verify` results, `bench` numbers,
   screenshots in `screenshots/`, open notes in `docs/issues/`.
2. **Write** `docs/reports/<Mx>.md` using the template below (blueprint Appendix D).
3. Keep it short: **at most 3 decisions**, each with a recommended option.
4. **Never claim something works unless it was run and observed.** Anything not run goes under
   "Masalah yang diketahui" or is marked `[ ]` with the reason.

## Template

```markdown
# Laporan Milestone <Mx> - <judul>

## Ringkasan (maks. 3 kalimat)

## Cara menjalankan
1. ...

## Yang selesai (sesuai Definition of Done)
- [x] ...
- [ ] ... (belum, alasannya)

## Screenshot
(fajar, siang, senja, malam, ketinggian 5 km, level jalan)

## Performa (laptop dev)
| Preset | Resolusi | FPS rata-rata | FPS 1% low | Tick sim |

## Masalah yang diketahui

## Keputusan yang dibutuhkan (maks. 3, dengan rekomendasi)

## Rencana milestone berikutnya
```

## Language and tone

- Report body: **Bahasa Indonesia**, short and concrete. No marketing language.
- Keep code identifiers, file paths, crate names and commands in English/original form.
- Run commands must be copy-paste ready for PowerShell on the owner's machine.
- Performance table: fill only rows actually measured. The dev laptop panel is 1920 × 1200, so a
  1440p row is "tidak bisa diuji di sini" — do not invent numbers.

## After writing

Tell the owner in chat (Bahasa Indonesia) that the build is ready to playtest, with the exact run
steps, then **stop and wait**. Do not start the next milestone without approval.

# Library Reclassification

Date: 2026-10-24
Status: agreed design, implemented in all four phases, not yet run on the real NAS

This document replaces the "Categories and Genre Facets" and "Organization
Preview and Future NAS Commit" parts of the original 2026-05-10 design. The NAS
library moves from the legacy hand-sorted folders to a forum-based genre
taxonomy, and grimoire becomes the tool that classifies games and moves the
files.

## Goals

- One primary category per game decides its NAS folder.
- Any number of genre codes per game, used for search and filtering only.
- grimoire suggests the category, the user confirms, grimoire moves the files.
- Moves are previewed, never overwrite, are logged, and can be undone.
- The migration is incremental: only confirmed games move.

## Taxonomy

### Folders

The library root holds these folders. The number prefix fixes the order in
Synology File Station. `99 其他(ETC)` always sorts last.

| Category id | Folder name | Label |
|---|---|---|
| `AVG` | `01 圖文冒險(AVG)` | 圖文冒險 |
| `ACT` | `02 動作格鬥(ACT)` | 動作格鬥 |
| `STG` | `03 射擊競技(STG)` | 射擊競技 |
| `RPG` | `04 角色扮演(RPG)` | 角色扮演 |
| `SLG` | `05 策略模擬(SLG)` | 策略模擬 |
| `SIM` | `06 經營養成(SIM)` | 經營養成 |
| `SPG` | `07 運動競速(SPG)` | 運動競速 |
| `PZL` | `08 益智解謎(PZL)` | 益智解謎 |
| `MUG` | `09 音樂節奏(MUG)` | 音樂節奏 |
| `HAG` | `10 動畫支向(HAG)` | 動畫支向 |
| `ETC` | `99 其他(ETC)` | 其他 |

`00 待整理` is the inbox for new downloads. It is not a category.

A category id is always also a genre code that belongs to that category, so a
game in `04 角色扮演(RPG)` can always carry the `RPG` code.

Folders are created only when the first file moves in. An unused category
never leaves an empty folder on the NAS.

### Genre codes

The vocabulary comes from the forum rules the user follows. It is a closed list
stored in the database and editable from Settings. Free-form labels go into
personal tags instead.

| Category | Codes |
|---|---|
| 01 圖文冒險 | ADV 冒險解謎, AVG 戀愛冒險, NVL 無支向電子小說, DEW 致鬱電波 |
| 02 動作格鬥 | ACT 動作, A-AVG 冒險, FTG 格鬥, ARPG 動作型角色扮演, HEG H逃脫, DIG 挖掘, DSG 下樓梯, SSG 隱蔽潛行 |
| 03 射擊競技 | FPS 第一人稱視角射擊, TPS 第三人稱視角射擊, STG 射擊, ASTG 彈幕射擊 |
| 04 角色扮演 | RPG 角色扮演, JRPG 半回合制, TRPG 桌遊扮演, CPG 抓寵育成 |
| 05 策略模擬 | SLG 策略遊戲, RTS 即時戰略, TD 塔防, TCG 卡牌對戰, QTE 即時反應, CLG 換裝, TBS 文字遊戲, T-RPG 游標戰鬥, SRPG 策略戰棋 |
| 06 經營養成 | EDU 戀愛養成, SIM 模擬經營, AGP 放置遊戲, WES 排泄系統, PPG 檢核文牒 |
| 07 運動競速 | RAC 賽車競速, RCG 競速, SPG 運動, PKG 跑酷 |
| 08 益智解謎 | PZL 益智, TAB 桌面遊戲, CAG 卡牌遊戲, MJG 麻將, SHG 驚悚遊戲, BSG 磚瓦粉碎, ERG 密室脫出, PBG 彈珠檯, TPG 打字遊戲, SKB 倉庫番, AA 見縫插針, SAN 精神健康值, SIK 西瓜合成, MWG 踩地雷, RT 俄羅斯方塊, MFC 對對碰 |
| 09 音樂節奏 | MUG 音樂, DAN 節奏類, VG 音聲類 |
| 10 動畫支向 | HAG 動畫遊戲, HAG² 動漫風3D動畫, HAG³ 寫實動畫, HAG⁴ AI擬真動畫, HAP 迷你像素, HAP² 像素動畫, LS 幻燈片 |
| 99 其他 | ETC 其他 |

Resolved differences from the user's first draft:

- HAG and T-HAG belong to 動畫支向, not 策略模擬.
- SIM belongs to 經營養成 only.
- HAP is 迷你像素 and belongs to 動畫支向, not 益智解謎.
- `H-ERG` is an alias of ERG 密室脫出.
- RPG belongs to 角色扮演 only. ARPG is 動作格鬥. T-RPG and SRPG are 策略模擬.
- The forum lists JRPG twice. It is one code.

### Variant codes

A stored code is `[prefix]<code>[level]`.

- **Level.** Any code takes a level from ² to ⁹, for example `SSG³`. Real
  works use levels outside 動畫支向, so levels are not limited to the codes the
  forum lists with one. The forum's named levels (HAG² 動漫風3D動畫, HAG³
  寫實動畫, HAG⁴ AI擬真動畫, HAP² 像素動畫) stay as rows with their own label
  and win over the generic reading. Any other level is shown as the base
  label plus the superscript, for example 隱蔽潛行³. Adding a row such as
  `SSG³` in Settings gives that level a name; it joins the SSG family.
- **Prefix.** `T-` means touch interaction and `X-` means partial or
  cross-section view. They apply only to codes marked as accepting a prefix:
  the 動畫支向 codes HAG, HAP and LS and their levels. Settings can mark more.
- Prefix and level combine, for example `T-HAG³`.
- `T-RPG` is its own code in 策略模擬. It is not "touch RPG". Exact codes are
  matched before prefix and level parsing.
- Codes are stored as the forum writes them. Input accepts plain digits, so
  `HAG2` becomes `HAG²` and `t-hag3` becomes `T-HAG³`.
- Each code has a family. HAG², T-HAG and HAG³ belong to the HAG family, and
  SSG³ to SSG. Searching `HAG` matches the whole family.

## Tags

There are three kinds of labels on a game:

- **Source tags.** Fetched from DLsite, VNDB or Steam. They are stored per
  source and shown as two-part pills, for example `ファンタジー｜DLsite`.
- **Genre codes.** The forum vocabulary above, stored on the work. Shown as
  pills in a different colour. The primary category is shown as its own pill
  with a star.
- **Personal tags.** Free text, unchanged by this work.

A work's genre codes always include at least one code from its primary
category. If none is given, the category id is added on confirm.

The search box matches title, filename, circle, external ids, source tags,
genre codes with family matching, and the primary category label.

### Source data per source

Today `game_works.source_tags` and `game_works.work_type` are overwritten by
whichever source was applied last. A new table keeps each source's raw tags and
work type separately. The suggestion engine reads the raw DLsite work type from
it, even after a VNDB link has changed the displayed type.

Existing rows are backfilled from the single external id each work has. A
one-off background re-fetch then refreshes every linked work at one request per
second. The re-fetch writes only the per-source tags and work type. It never
touches titles, circles, descriptions or images the user may have edited.

## Classification

### Unit of classification

Classification lives on the game work, not on the file. Every file linked to a
work, including patches and DLC, goes to the same category. A file that is not
linked to a work cannot be classified. Organize already handles linking or
creating a manual entry first.

`inventory_items.primary_category` keeps the values set under the old category
list. They are no longer written and serve only as hints.

### Suggestions

grimoire computes a suggestion for every work without a confirmed category. A
suggestion has a category, a list of codes, a confident flag and the reasons.

**Work type rules.** Editable in Settings. Seed values:

| Source | Work type | Code | Confident |
|---|---|---|---|
| DLsite | ACN 動作 | ACT | yes |
| DLsite | STG 射擊 | STG | yes |
| DLsite | RPG 角色扮演 | RPG | yes |
| DLsite | ADV 冒險 | ADV | yes |
| DLsite | DNV 電子小說 | NVL | yes |
| DLsite | PZL 益智 | PZL | yes |
| DLsite | QIZ 問答 | PZL | yes |
| DLsite | TBL 桌上遊戲 | TAB | yes |
| DLsite | TYP 打字 | TPG | yes |
| DLsite | SLN 模擬 | look at tags | no |
| DLsite | ETC 其他遊戲 | look at tags | no |
| DLsite | non-game types such as MNG, ICG, SOU, MOV, MUS, TOL | ETC | yes |
| VNDB | ADV | AVG | yes |
| Steam | STEAM | look at tags | no |

**Keyword rules.** Editable in Settings. A keyword matches a source tag by
case-insensitive substring and adds a code. A rule can be limited to one
source. A rule can also be marked "contests type". Only such keywords make a
suggestion need review when they point to a different category than the work
type. 調教 is a common DLsite content tag, so without this flag most confident
RPGs would land in the queue. Seed values cover the groups agreed in review:

- 戦略, ストラテジー, 策略, 戰略 give SLG. タワーディフェンス, 塔防 give TD.
  カードバトル, 卡牌對戰 give TCG.
- 経営, 經營 give SIM. 育成, 養成, 調教 give EDU. 放置ゲーム, 放置遊戲 give
  AGP. Bare 放置 is not a keyword because DLsite uses 放置プレイ as a content
  tag.
- アニメーション, Live2D, 動畫 give HAG. タッチ, 觸摸 give T-HAG.
- アクションRPG gives ARPG. シミュレーションRPG and SRPG give SRPG. These
  three are the only seeds marked "contests type".
- VNDB only: `No Choices` gives NVL.
- Steam only, using the Traditional Chinese genre names: 動作 ACT, 冒險 ADV,
  角色扮演 RPG, 策略 SLG, 模擬 SIM, 競速 RAC, 運動 SPG.

The seed keywords have not been checked against the live library, which was
unreachable during design. Settings lists the most common source tags on
unclassified works so the real keyword list can be filled in from data.

**Hints.**

| Hint | Value | Categories |
|---|---|---|
| Old primary category | Visual Novel | AVG |
| Old primary category | Action | ACT |
| Old primary category | RPG | RPG |
| Old primary category | Strategy | SLG |
| Old primary category | Simulation, 3D | no hint |
| Legacy folder | `RPG` | RPG |
| Legacy folder | `ADV` | AVG |
| Legacy folder | `ACT` | ACT |
| Legacy folder | `SIM+SLG` | SLG or SIM |
| Legacy folder | `ETC`, `待分類`, `未分類`, `LunaSoft` | no hint |

**Algorithm.**

1. For each source the work has data from, look up the work type rule. A rule
   with a code proposes that code and its category.
2. Run the keyword rules over each source's tags. A keyword code in the same
   category as that source's work type code replaces it as a refinement, for
   example FTG replacing ACT, or NVL replacing AVG for VNDB. A keyword code in
   a different category is kept as an extra code. When that keyword is marked
   "contests type", the suggestion is not confident. This is how a DLsite RPG
   that looks like an ARPG or SRPG ends up in the queue.
3. If no work type rule gave a category, the keyword codes decide when they all
   point to one category. Otherwise the hints decide when they agree on one
   category. Such suggestions are never confident.
4. A confident suggestion needs a confident work type rule, no contesting
   keyword, agreement between sources, and every non-empty hint containing the
   suggested category. A legacy folder that is already a new category folder
   also counts as a hint for that category.
5. Works without any source data get a suggestion from hints only, never
   confident.

### Confirming

- **Classify view.** Lists confident suggestions in a table. The user ticks
  rows, or selects all, and confirms them in one action.
- **Organize queue.** A new "Needs category" filter holds works whose
  suggestion is not confident. The item panel shows the suggestion, its
  reasons, a category picker and the code pills, prefilled from the
  suggestion. The code picker lists each code once. Clicking a code pill
  opens its Prefix and Level toggles; combinations the vocabulary does not
  accept are disabled. Adding a code to an empty list also sets the primary
  category to that code's category.
- Any manual change of category or codes counts as confirmation.
- Confirming does not move files.

## NAS layout

```
<category folder>/<circle>/<original file name>
<category folder>/<circle>/<work title>/<original file name>
```

- **Circle.** The work's circle field. For VNDB and Steam works this already
  holds the first developer. Works without one go under `未知社團`.
- **Work folder.** Added when a work has two or more non-missing files.
  Versions, patches and DLC all count. A Chinese edition with its own RJ code is
  a different work unless the user links it to the original work.
- **Work folder name.** The work's display title in grimoire.
- **No auto-flatten.** A file already inside its work folder stays there even
  if the work drops back to one file.
- **No shared work folders.** If two works of one circle and category map to
  the same folder name, for example `Title?` and `Title:`, their files are
  blocked until one title is changed.
- **File names never change.**
- **Segment safety.** Characters invalid on Windows or SMB become `_`.
  Leading and trailing spaces and trailing dots are trimmed. Each segment is
  capped at 200 bytes on a character boundary. An encrypted Synology share
  has a lower limit of 143 bytes. Such a move fails cleanly and is reported.
- **LunaSoft.** The legacy `LunaSoft` folder dissolves naturally. Its games go to
  their genre folders under a `LunaSoft` circle folder.
- **待分類 and 未分類.** These are ordinary legacy folders.
- **Excluded from moves.** The asset cache directory, `#recycle`, and any file
  that is not a scanned zip or rar.

## Moving files

### Plan, preview, execute

- **Plan.** Every non-missing file whose work has a confirmed category and whose
  current path is not an accepted target. An accepted target is either the flat
  path, or the work-folder path when no-auto-flatten applies.
- **Preview.** For the files the user selected, the preview shows source,
  target, conflicts, and the legacy folders that will be removed.
- **Execute.** The server recomputes the plan from the database for the
  selected file ids and never trusts paths from the client. The batch is
  recorded first, then each file is moved and its row updated, and finally
  emptied folders are pruned.

### Safety

- **No overwrite, ever.** POSIX rename silently replaces an existing target,
  so a move is a hard link followed by an unlink. Link creation fails
  atomically when the target exists. On a filesystem without hard links the
  writer checks that the target is missing and then renames.
- **Conflicts skip the file.** A target that exists on disk, two files with
  the same target, or another inventory row already at the target all skip
  the file. Skipped files appear in the batch report.
- **Failures are isolated.** One failed file does not stop the batch. If the
  database update fails after a move, the file is moved back.
- **One writer at a time.** Scan, execute and undo share one lock, so a scan
  never sees a half-moved library.
- **Interruption-safe.** A batch runs in a detached task, so a closed browser
  tab cannot stop it between a move and its record. Each operation is logged
  as pending before its file is touched. After a crash, restart or lost
  database connection, the pending operations are reconciled against the
  disk. This happens at startup and before every batch or undo. The database
  row follows wherever the file really is.
- **Unclear database results are re-read.** If an update fails after a move,
  the row is read again before anything moves back, because the commit may
  have landed.
- **Rollback never deletes the last copy.** If removing the source fails
  after the hard link, the link is only rolled back while the source still
  exists.
- **The rename fallback stays exclusive.** It is used only when the share
  refuses hard links. It first claims the target with an exclusive create,
  which the NFS server decides, then renames over that empty placeholder.
- **Legacy location is kept.** A moved row keeps its original legacy
  location. Rescans match the new path, so links and metadata stay intact.

### Undo

Each batch keeps every operation with its source, target and outcome. Undo
replays the done operations in reverse with the same no-overwrite move. A file
whose original location is taken again stays where it is and is reported.
Folders emptied by the undo are pruned the same way.

### Pruning

After a batch, each source folder is checked upward towards the library root. A
folder is removed when it holds only metadata. Metadata files are `.DS_Store`,
`Thumbs.db`, `desktop.ini` and `._*`. Metadata folders are `@eaDir` and
`.AppleDouble`, and they count only when no zip or rar is anywhere inside them.
A zip or rar is never metadata, whatever its name. Metadata is removed together
with the folder.

grimoire never creates a folder that looks like metadata or is hidden. A title
such as `._Foo` becomes the folder `_._Foo`.

These folders are never removed: the library root, category folders, the
`00 待整理` inbox, the asset cache directory, and `#recycle`.

### Other files

The scanner only knows zip and rar. Settings has a report of everything else
under the library root. A directory whose whole subtree has no zip or rar is
collapsed into one line with its file count and size. The report is
read-only. Those files are handled by hand.

## Data model

New migrations, one per phase:

- **`0008_reclassification_vocabulary.sql`.**
  - Adds `categories` and `genre_codes`, with the seed data above.
  - Adds `work_source_data`, keyed by game work and source, holding tags,
    work type, work type label and the fetch time. It is backfilled from
    `game_works`.
  - Adds `game_works.category_confirmed_at`.
  - Adds a foreign key from `game_works.primary_category` to `categories`.
- **`0009_classification_rules.sql`.** Adds `work_type_rules` and
  `keyword_rules`, with the seed data above. Rule codes are validated by the
  API instead of a foreign key, because variant codes such as T-HAG and SSG³
  are not rows of `genre_codes`.
- **`0010_move_batches.sql`.** Adds `move_batches` and `move_ops`.

`game_works.source_tags` is no longer read and stays only as a backup until a
later cleanup drops it.

## API

| Method and path | Purpose |
|---|---|
| `GET /api/vocabulary` | categories and genre codes |
| `POST /api/vocabulary/codes` | add or edit a code |
| `DELETE /api/vocabulary/codes/{code}` | remove an unused code |
| `POST /api/classification/confirm` | set category and codes on one or more works |
| `GET /api/classification/suggestions` | suggestions for unconfirmed works |
| `GET /api/classification/rules` | list work type and keyword rules |
| `POST /api/classification/rules/work-type` | add or edit a work type rule |
| `DELETE /api/classification/rules/work-type/{source}/{type}` | remove a work type rule |
| `POST /api/classification/rules/keyword` | add or edit a keyword rule |
| `DELETE /api/classification/rules/keyword/{id}` | remove a keyword rule |
| `GET /api/classification/common-tags` | frequent source tags on unclassified works |
| `POST /api/sources/refetch` | start the background re-fetch |
| `GET /api/sources/refetch` | re-fetch progress |
| `GET /api/organize/plan` | every pending move |
| `POST /api/organize/preview` | preview for selected files |
| `POST /api/organize/execute` | run a batch for selected files |
| `GET /api/organize/batches` | batch history |
| `GET /api/organize/batches/{id}` | one batch with its operations |
| `POST /api/organize/batches/{id}/undo` | undo a batch |
| `GET /api/organize/inbox` | whether `00 待整理` exists |
| `POST /api/organize/inbox` | create `00 待整理` |
| `GET /api/organize/other-files` | report of non-archive files |

The `edit-item` endpoint that wrote the old per-file category is removed.

## Rollout and verification

1. Run the whole flow on the fixture library and a throwaway Postgres,
   including undo. Done on 2026-10-24 against PostgreSQL 16, see below.
2. On the real NAS, only open the Move preview. Do not execute. The user
   reviews it.
3. The user takes a Synology snapshot of the games share and a fresh dump of
   the grimoire schema. The cluster already has a nightly dump, but a dump
   taken right before the first batch is safer. grimoire's export endpoint is
   still a stub that returns an empty list, so it is not a backup.
4. Execute a first batch of about ten files from one category. Check it in File
   Station, undo it once, then execute it again.
5. Continue one category at a time.

## Implementation plan

Each phase is usable on its own. Phases 1 and 2 never touch NAS files.

### Phase 1: vocabulary and tags

1. Migration 0008 with seed data and backfill.
2. `domain::vocabulary` and `domain::genre_code`: code normalisation, aliases,
   prefixes, superscripts, families. Unit tested.
3. `apply_detail` upserts `work_source_data` for the applied source.
4. `GET /api/vocabulary` and the code add, edit and delete endpoints.
5. `POST /api/classification/confirm` with validation, normalisation and the
   "at least one code from the category" rule.
6. Library response: work-level category and codes, `categoryConfirmed`,
   per-source tags, and the file path.
7. Re-fetch job and its endpoints.
8. UI: tag pills, code pills, category pill with a star, category picker and
   code editor in the item panel, Codes filter, search with family matching,
   and Settings with the code list and the re-fetch button.

### Phase 2: suggestions and batch confirm

1. Migration 0009 with seed rules.
2. `domain::category_suggestion`: the suggestion algorithm as a pure
   derivation. Unit tested.
3. Suggestions, rules and common tags endpoints.
4. UI: Classify view with batch confirm, the Organize "Needs category" filter,
   the suggestion box in the item panel, and the rules editor and common tags
   list in Settings.

### Phase 3: moves

1. Migration 0010.
2. `domain::file_placement` and `domain::move_plan`: target path planning with
   the no-auto-flatten rule, segment sanitising and conflicts. Unit tested.
3. `infrastructure::storage` (`FsLibraryFiles`, `FsLibraryFolders`):
   no-overwrite move and pruning. Tested on temporary directories. The move log,
   undo and reconciliation rules live in `domain::move_batch`.
4. Plan, preview, execute, batches and undo endpoints, with the shared library
   lock also taken by scan.
5. UI: Move view with per-category selection, preview, execute, and batch
   history with undo.

### Phase 4: inbox and report

1. Inbox status and create endpoints, plus a button in the Move view.
2. Other-files report endpoint and its Settings section.

### Testing

- Rust unit tests for vocabulary, classification, planning and the writer.
- An end-to-end run against Docker Postgres and a copy of the fixture library:
  scan, manual entries, confirm, plan, execute, rescan, undo.
- `tsc` and the Vite build for the web app.

## Verification log

2026-10-24, on a copy of `fixtures/library` and PostgreSQL 16 in Docker:

- Migrations 0001 to 0007 were applied first, with works and old category
  values inserted, then 0008 to 0010. The backfill attributed each work's tags
  to its source and skipped the manual entry.
- Confirm, vocabulary and rule endpoints returned the expected 400, 404 and 409
  errors for unknown codes, unknown works, used codes and category codes.
- The re-fetch refreshed VNDB tags without changing the edited title, and
  reported an unknown DLsite id as a failure.
- A batch moved four files. Two files of one work went into a work folder. The
  LunaSoft folder dissolved into `04 角色扮演(RPG)/LunaSoft`. A file whose
  target was occupied was skipped. Legacy folders holding only `@eaDir` and
  `.DS_Store` were removed, exactly as the preview listed. A rescan marked
  nothing missing.
- Undo moved three files back, kept the one whose original path had been taken
  again, pruned the emptied new folders, and refused a second undo.

An independent review of the move code then found seven problems. All seven
were fixed:

- interruption could desynchronise the disk and the database
- the rollback could delete the last copy
- the rename fallback could overwrite a file
- metadata-like names could get real archives pruned
- an ambiguous database error could trigger a wrong move-back
- two works could share a work folder
- the batch log could omit pruned folders

The scenario was rerun on a fresh database and added three checks:

- **Interrupted batch.** One file had been moved but not recorded. After a
  restart it was recovered, and its row was updated. The operation that never
  moved stayed at the original path. The recovered batch could be undone.
- **Colliding titles.** Two works whose titles collide were blocked.
- **Metadata-like title.** A title that looks like metadata got a safe folder
  name.

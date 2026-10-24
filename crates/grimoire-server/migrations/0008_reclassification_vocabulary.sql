SET search_path TO grimoire, public;

-- Primary categories: one per NAS folder. The id is also a genre code that
-- belongs to the category, so a work can always carry its category's code.
CREATE TABLE IF NOT EXISTS categories (
    id          text PRIMARY KEY,
    label       text NOT NULL,
    folder_name text NOT NULL UNIQUE,
    sort_order  integer NOT NULL
);

INSERT INTO categories (id, label, folder_name, sort_order) VALUES
    ('AVG', '圖文冒險', '01 圖文冒險(AVG)', 1),
    ('ACT', '動作格鬥', '02 動作格鬥(ACT)', 2),
    ('STG', '射擊競技', '03 射擊競技(STG)', 3),
    ('RPG', '角色扮演', '04 角色扮演(RPG)', 4),
    ('SLG', '策略模擬', '05 策略模擬(SLG)', 5),
    ('SIM', '經營養成', '06 經營養成(SIM)', 6),
    ('SPG', '運動競速', '07 運動競速(SPG)', 7),
    ('PZL', '益智解謎', '08 益智解謎(PZL)', 8),
    ('MUG', '音樂節奏', '09 音樂節奏(MUG)', 9),
    ('HAG', '動畫支向', '10 動畫支向(HAG)', 10),
    ('ETC', '其他', '99 其他(ETC)', 99)
ON CONFLICT (id) DO NOTHING;

-- Closed genre-code vocabulary from the forum rules. `family` groups variants
-- (HAG² belongs to HAG) for search; `accepts_prefix` allows the T- (touch) and
-- X- (partial / cross-section) prefixes, which only exist for 動畫支向 codes.
CREATE TABLE IF NOT EXISTS genre_codes (
    code           text PRIMARY KEY,
    label          text NOT NULL,
    category_id    text NOT NULL REFERENCES categories(id),
    family         text NOT NULL,
    accepts_prefix boolean NOT NULL DEFAULT false,
    aliases        jsonb NOT NULL DEFAULT '[]',
    sort_order     integer NOT NULL
);

INSERT INTO genre_codes (code, label, category_id, family, accepts_prefix, aliases, sort_order) VALUES
    ('ADV',   '冒險解謎',         'AVG', 'ADV',   false, '[]', 101),
    ('AVG',   '戀愛冒險',         'AVG', 'AVG',   false, '[]', 102),
    ('NVL',   '無支向電子小說',   'AVG', 'NVL',   false, '[]', 103),
    ('DEW',   '致鬱電波',         'AVG', 'DEW',   false, '[]', 104),
    ('ACT',   '動作',             'ACT', 'ACT',   false, '[]', 201),
    ('A-AVG', '冒險',             'ACT', 'A-AVG', false, '[]', 202),
    ('FTG',   '格鬥',             'ACT', 'FTG',   false, '[]', 203),
    ('ARPG',  '動作型角色扮演',   'ACT', 'ARPG',  false, '[]', 204),
    ('HEG',   'H逃脫',            'ACT', 'HEG',   false, '[]', 205),
    ('DIG',   '挖掘',             'ACT', 'DIG',   false, '[]', 206),
    ('DSG',   '下樓梯',           'ACT', 'DSG',   false, '[]', 207),
    ('SSG',   '隱蔽潛行',         'ACT', 'SSG',   false, '[]', 208),
    ('FPS',   '第一人稱視角射擊', 'STG', 'FPS',   false, '[]', 301),
    ('TPS',   '第三人稱視角射擊', 'STG', 'TPS',   false, '[]', 302),
    ('STG',   '射擊',             'STG', 'STG',   false, '[]', 303),
    ('ASTG',  '彈幕射擊',         'STG', 'ASTG',  false, '[]', 304),
    ('RPG',   '角色扮演',         'RPG', 'RPG',   false, '[]', 401),
    ('JRPG',  '半回合制',         'RPG', 'JRPG',  false, '[]', 402),
    ('TRPG',  '桌遊扮演',         'RPG', 'TRPG',  false, '[]', 403),
    ('CPG',   '抓寵育成',         'RPG', 'CPG',   false, '[]', 404),
    ('SLG',   '策略遊戲',         'SLG', 'SLG',   false, '[]', 501),
    ('RTS',   '即時戰略',         'SLG', 'RTS',   false, '[]', 502),
    ('TD',    '塔防',             'SLG', 'TD',    false, '[]', 503),
    ('TCG',   '卡牌對戰',         'SLG', 'TCG',   false, '[]', 504),
    ('QTE',   '即時反應',         'SLG', 'QTE',   false, '[]', 505),
    ('CLG',   '換裝',             'SLG', 'CLG',   false, '[]', 506),
    ('TBS',   '文字遊戲',         'SLG', 'TBS',   false, '[]', 507),
    ('T-RPG', '游標戰鬥',         'SLG', 'T-RPG', false, '[]', 508),
    ('SRPG',  '策略戰棋',         'SLG', 'SRPG',  false, '[]', 509),
    ('EDU',   '戀愛養成',         'SIM', 'EDU',   false, '[]', 601),
    ('SIM',   '模擬經營',         'SIM', 'SIM',   false, '[]', 602),
    ('AGP',   '放置遊戲',         'SIM', 'AGP',   false, '[]', 603),
    ('WES',   '排泄系統',         'SIM', 'WES',   false, '[]', 604),
    ('PPG',   '檢核文牒',         'SIM', 'PPG',   false, '[]', 605),
    ('RAC',   '賽車競速',         'SPG', 'RAC',   false, '[]', 701),
    ('RCG',   '競速',             'SPG', 'RCG',   false, '[]', 702),
    ('SPG',   '運動',             'SPG', 'SPG',   false, '[]', 703),
    ('PKG',   '跑酷',             'SPG', 'PKG',   false, '[]', 704),
    ('PZL',   '益智',             'PZL', 'PZL',   false, '[]', 801),
    ('TAB',   '桌面遊戲',         'PZL', 'TAB',   false, '[]', 802),
    ('CAG',   '卡牌遊戲',         'PZL', 'CAG',   false, '[]', 803),
    ('MJG',   '麻將',             'PZL', 'MJG',   false, '[]', 804),
    ('SHG',   '驚悚遊戲',         'PZL', 'SHG',   false, '[]', 805),
    ('BSG',   '磚瓦粉碎',         'PZL', 'BSG',   false, '[]', 806),
    ('ERG',   '密室脫出',         'PZL', 'ERG',   false, '["H-ERG"]', 807),
    ('PBG',   '彈珠檯',           'PZL', 'PBG',   false, '[]', 808),
    ('TPG',   '打字遊戲',         'PZL', 'TPG',   false, '[]', 809),
    ('SKB',   '倉庫番',           'PZL', 'SKB',   false, '[]', 810),
    ('AA',    '見縫插針',         'PZL', 'AA',    false, '[]', 811),
    ('SAN',   '精神健康值',       'PZL', 'SAN',   false, '[]', 812),
    ('SIK',   '西瓜合成',         'PZL', 'SIK',   false, '[]', 813),
    ('MWG',   '踩地雷',           'PZL', 'MWG',   false, '[]', 814),
    ('RT',    '俄羅斯方塊',       'PZL', 'RT',    false, '[]', 815),
    ('MFC',   '對對碰',           'PZL', 'MFC',   false, '[]', 816),
    ('MUG',   '音樂',             'MUG', 'MUG',   false, '[]', 901),
    ('DAN',   '節奏類',           'MUG', 'DAN',   false, '[]', 902),
    ('VG',    '音聲類',           'MUG', 'VG',    false, '[]', 903),
    ('HAG',   '動畫遊戲',         'HAG', 'HAG',   true,  '[]', 1001),
    ('HAG²',  '動漫風3D動畫',     'HAG', 'HAG',   true,  '[]', 1002),
    ('HAG³',  '寫實動畫',         'HAG', 'HAG',   true,  '[]', 1003),
    ('HAG⁴',  'AI擬真動畫',       'HAG', 'HAG',   true,  '[]', 1004),
    ('HAP',   '迷你像素',         'HAG', 'HAP',   true,  '[]', 1005),
    ('HAP²',  '像素動畫',         'HAG', 'HAP',   true,  '[]', 1006),
    ('LS',    '幻燈片',           'HAG', 'LS',    true,  '[]', 1007),
    ('ETC',   '其他',             'ETC', 'ETC',   false, '[]', 9901)
ON CONFLICT (code) DO NOTHING;

-- Raw per-source metadata. game_works.source_tags / work_type are overwritten
-- by whichever source was applied last; this keeps each source's own values so
-- tags can show their provenance and the classifier can read DLsite's type.
CREATE TABLE IF NOT EXISTS work_source_data (
    game_work_id    uuid NOT NULL REFERENCES game_works(id) ON DELETE CASCADE,
    source          text NOT NULL CHECK (source IN ('dlsite', 'vndb', 'steam')),
    tags            jsonb NOT NULL DEFAULT '[]',
    work_type       text,
    work_type_label text,
    fetched_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (game_work_id, source)
);

-- Backfill: the existing code only ever links a work to one source, so the
-- external id column that is set tells where its tags came from.
INSERT INTO work_source_data (game_work_id, source, tags, work_type, work_type_label, fetched_at)
SELECT id,
       CASE
           WHEN dlsite_work_id IS NOT NULL THEN 'dlsite'
           WHEN vndb_id IS NOT NULL THEN 'vndb'
           ELSE 'steam'
       END,
       COALESCE(source_tags, '[]'),
       work_type,
       work_type_label,
       COALESCE(enriched_at, updated_at)
  FROM game_works
 WHERE dlsite_work_id IS NOT NULL OR vndb_id IS NOT NULL OR steam_app_id IS NOT NULL
ON CONFLICT (game_work_id, source) DO NOTHING;

-- Classification moves to the work. NULL means "not confirmed yet".
ALTER TABLE game_works
    ADD COLUMN IF NOT EXISTS category_confirmed_at timestamptz;

-- game_works.primary_category was never written before this migration, but
-- clear anything that would violate the new foreign key just in case.
UPDATE game_works
   SET primary_category = NULL
 WHERE primary_category IS NOT NULL
   AND primary_category NOT IN (SELECT id FROM categories);

ALTER TABLE game_works
    DROP CONSTRAINT IF EXISTS game_works_primary_category_fkey;
ALTER TABLE game_works
    ADD CONSTRAINT game_works_primary_category_fkey
    FOREIGN KEY (primary_category) REFERENCES categories(id);

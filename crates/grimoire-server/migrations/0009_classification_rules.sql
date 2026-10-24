SET search_path TO grimoire, public;

-- Source work type → genre code. A NULL code means "decide from the tags".
-- `confident` marks rules whose suggestions can be batch-confirmed.
CREATE TABLE IF NOT EXISTS work_type_rules (
    source     text NOT NULL CHECK (source IN ('dlsite', 'vndb', 'steam')),
    work_type  text NOT NULL,
    code       text,
    confident  boolean NOT NULL DEFAULT false,
    PRIMARY KEY (source, work_type)
);

-- Source tag keyword (case-insensitive substring) → genre code. A NULL source
-- applies to every source. `contests_type` marks keywords that describe the
-- genre itself (アクションRPG), so landing in another category than the work
-- type makes the suggestion not confident. Other keywords only add codes.
--
-- Codes are validated against the vocabulary by the API rather than by a
-- foreign key because variant codes such as T-HAG are not rows of genre_codes.
CREATE TABLE IF NOT EXISTS keyword_rules (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    source        text CHECK (source IN ('dlsite', 'vndb', 'steam')),
    keyword       text NOT NULL,
    code          text NOT NULL,
    contests_type boolean NOT NULL DEFAULT false,
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS keyword_rules_unique_idx
    ON keyword_rules (COALESCE(source, ''), lower(keyword));

INSERT INTO work_type_rules (source, work_type, code, confident) VALUES
    ('dlsite', 'ACN', 'ACT', true),
    ('dlsite', 'STG', 'STG', true),
    ('dlsite', 'RPG', 'RPG', true),
    ('dlsite', 'ADV', 'ADV', true),
    ('dlsite', 'DNV', 'NVL', true),
    ('dlsite', 'PZL', 'PZL', true),
    ('dlsite', 'QIZ', 'PZL', true),
    ('dlsite', 'TBL', 'TAB', true),
    ('dlsite', 'TYP', 'TPG', true),
    ('dlsite', 'SLN', NULL, false),
    ('dlsite', 'ETC', NULL, false),
    -- DLsite non-game work types: manga, comics, CG, voice, music, video,
    -- tools, assets, novels.
    ('dlsite', 'MNG', 'ETC', true),
    ('dlsite', 'SCM', 'ETC', true),
    ('dlsite', 'WBT', 'ETC', true),
    ('dlsite', 'VCM', 'ETC', true),
    ('dlsite', 'ICG', 'ETC', true),
    ('dlsite', 'SOU', 'ETC', true),
    ('dlsite', 'MUS', 'ETC', true),
    ('dlsite', 'MOV', 'ETC', true),
    ('dlsite', 'TOL', 'ETC', true),
    ('dlsite', 'IMT', 'ETC', true),
    ('dlsite', 'AMT', 'ETC', true),
    ('dlsite', 'NRE', 'ETC', true),
    ('dlsite', 'ET3', 'ETC', true),
    ('vndb', 'ADV', 'AVG', true),
    ('steam', 'STEAM', NULL, false)
ON CONFLICT (source, work_type) DO NOTHING;

INSERT INTO keyword_rules (source, keyword, code, contests_type) VALUES
    (NULL, '戦略', 'SLG', false),
    (NULL, 'ストラテジー', 'SLG', false),
    (NULL, '策略', 'SLG', false),
    (NULL, '戰略', 'SLG', false),
    (NULL, 'タワーディフェンス', 'TD', false),
    (NULL, '塔防', 'TD', false),
    (NULL, 'カードバトル', 'TCG', false),
    (NULL, '卡牌對戰', 'TCG', false),
    (NULL, '経営', 'SIM', false),
    (NULL, '經營', 'SIM', false),
    (NULL, '育成', 'EDU', false),
    (NULL, '養成', 'EDU', false),
    (NULL, '調教', 'EDU', false),
    (NULL, '放置ゲーム', 'AGP', false),
    (NULL, '放置遊戲', 'AGP', false),
    (NULL, 'アニメーション', 'HAG', false),
    (NULL, 'Live2D', 'HAG', false),
    (NULL, '動畫', 'HAG', false),
    (NULL, 'タッチ', 'T-HAG', false),
    (NULL, '觸摸', 'T-HAG', false),
    (NULL, 'アクションRPG', 'ARPG', true),
    (NULL, 'シミュレーションRPG', 'SRPG', true),
    (NULL, 'SRPG', 'SRPG', true),
    ('vndb', 'No Choices', 'NVL', false),
    ('steam', '動作', 'ACT', false),
    ('steam', '冒險', 'ADV', false),
    ('steam', '角色扮演', 'RPG', false),
    ('steam', '模擬', 'SIM', false),
    ('steam', '競速', 'RAC', false),
    ('steam', '運動', 'SPG', false)
ON CONFLICT DO NOTHING;

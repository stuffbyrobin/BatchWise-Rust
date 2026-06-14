-- Seed: ~15 common yeast strains into the system tenant. Idempotent via ON CONFLICT (id) DO NOTHING.

INSERT INTO yeasts (id, tenant_id, name, manufacturer, product_code, type, form, attenuation_min_pct, attenuation_max_pct, temp_min_c, temp_max_c, flocculation, notes)
VALUES
    ('22222222-2222-2222-2222-000000000001', '00000000-0000-0000-0000-000000000000', 'American Ale',          'Wyeast',    '1056', 'ale',    'liquid', 73, 77, 16, 22, 'low',    'Clean, versatile American ale yeast.'),
    ('22222222-2222-2222-2222-000000000002', '00000000-0000-0000-0000-000000000000', 'American Ale US-05',    'Fermentis', 'US-05', 'ale',    'dry',    73, 77, 15, 24, 'medium', 'Dry version of WLP001/1056. Versatile and reliable.'),
    ('22222222-2222-2222-2222-000000000003', '00000000-0000-0000-0000-000000000000', 'California Ale',        'White Labs','WLP001','ale',    'liquid', 73, 80, 17, 22, 'medium', 'Clean, crisp fermentation. Industry standard for American ales.'),
    ('22222222-2222-2222-2222-000000000004', '00000000-0000-0000-0000-000000000000', 'London ESB Ale',        'Wyeast',    '1968', 'ale',    'liquid', 67, 71, 17, 22, 'high',   'Very high flocculation; rich malt character. Classic ESB yeast.'),
    ('22222222-2222-2222-2222-000000000005', '00000000-0000-0000-0000-000000000000', 'London Ale III',        'White Labs','WLP002','ale',    'liquid', 63, 70, 18, 22, 'high',   'Produces a soft, almost sweet malt profile.'),
    ('22222222-2222-2222-2222-000000000006', '00000000-0000-0000-0000-000000000000', 'Bohemian Lager',        'Wyeast',    '2124', 'lager',  'liquid', 71, 75, 8,  12, 'medium', 'Classic Bohemian lager character; clean and malty.'),
    ('22222222-2222-2222-2222-000000000007', '00000000-0000-0000-0000-000000000000', 'Belgian Ale T-58',      'Fermentis', 'T-58',  'ale',    'dry',    74, 78, 15, 24, 'medium', 'Spicy and fruity Belgian character. Good for saisons.'),
    ('22222222-2222-2222-2222-000000000008', '00000000-0000-0000-0000-000000000000', 'Kveik Voss',            'Lallemand', NULL,    'ale',    'dry',    75, 82, 25, 40, 'high',   'Norwegian farmhouse kveik. Extremely fast fermentation at high temps.'),
    ('22222222-2222-2222-2222-000000000009', '00000000-0000-0000-0000-000000000000', 'Belgian Witbier',       'Wyeast',    '3944', 'ale',    'liquid', 72, 76, 16, 22, 'low',    'Tart, spicy witbier character.'),
    ('22222222-2222-2222-2222-000000000010', '00000000-0000-0000-0000-000000000000', 'Weizen Hefeweizen',     'White Labs','WLP300','ale',    'liquid', 72, 76, 18, 24, 'low',    'Classic Bavarian hefeweizen. Banana and clove aromatics.'),
    ('22222222-2222-2222-2222-000000000011', '00000000-0000-0000-0000-000000000000', 'Saflager W-34/70',      'Fermentis', 'W-34/70','lager', 'dry',    71, 76, 9,  15, 'high',   'Most widely used lager yeast worldwide.'),
    ('22222222-2222-2222-2222-000000000012', '00000000-0000-0000-0000-000000000000', 'Irish Ale',             'Wyeast',    '1084', 'ale',    'liquid', 71, 75, 16, 22, 'medium', 'Slight fruitiness; classic dry Irish stout character.'),
    ('22222222-2222-2222-2222-000000000013', '00000000-0000-0000-0000-000000000000', 'Belgian Abbey II',      'Wyeast',    '1762', 'ale',    'liquid', 73, 77, 18, 24, 'medium', 'Rich, malty Belgian character with dark-fruit esters.'),
    ('22222222-2222-2222-2222-000000000014', '00000000-0000-0000-0000-000000000000', 'Saaz Lager',            'White Labs','WLP830','lager',  'liquid', 74, 79, 8,  14, 'medium', 'Traditional German lager. Smooth and clean.'),
    ('22222222-2222-2222-2222-000000000015', '00000000-0000-0000-0000-000000000000', 'Champagne Yeast',       'Lallemand', 'EC-1118','other', 'dry',    90,100, 10, 30, 'low',    'Very high attenuation. Suitable for cider and high-gravity finishing.')
ON CONFLICT (id) DO NOTHING;

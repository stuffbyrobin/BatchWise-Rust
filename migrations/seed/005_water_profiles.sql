INSERT INTO water_profiles (tenant_id, name, description, calcium_ppm, magnesium_ppm, sodium_ppm, sulfate_ppm, chloride_ppm, bicarbonate_ppm)
VALUES
    ('00000000-0000-0000-0000-000000000000', 'Burton on Trent', 'Classic English IPA water. High sulfate and calcium.',   295, 45, 55, 725, 25, 300),
    ('00000000-0000-0000-0000-000000000000', 'Dublin',          'Stout-friendly. High bicarbonate, moderate minerals.',    115,  4, 12,  55, 19, 319),
    ('00000000-0000-0000-0000-000000000000', 'Dortmund',        'Export lager profile. Balanced and mineralised.',         225, 40, 60, 120, 60, 180),
    ('00000000-0000-0000-0000-000000000000', 'Edinburgh',       'Scottish ale water. Moderate all round.',                 120, 25, 55, 140, 60, 225),
    ('00000000-0000-0000-0000-000000000000', 'London',          'Porter and mild. Moderate bicarbonate.',                   52, 32, 86,  32, 34, 104),
    ('00000000-0000-0000-0000-000000000000', 'Munich',          'Helles and dark lager. Soft, low sulfate.',                75, 18,  2,  10,  2, 150),
    ('00000000-0000-0000-0000-000000000000', 'Pilsen',          'Pilsner lager. Very soft, almost RO quality.',              7,  2,  2,   5,  5,  15),
    ('00000000-0000-0000-0000-000000000000', 'Vienna',          'Vienna lager. Balanced mid-range profile.',               200, 60,  8, 125, 12, 120),
    ('00000000-0000-0000-0000-000000000000', 'RO / Distilled',  'Reverse osmosis or distilled water. Zero minerals.',        0,  0,  0,   0,  0,   0)
ON CONFLICT (tenant_id, name) DO NOTHING;

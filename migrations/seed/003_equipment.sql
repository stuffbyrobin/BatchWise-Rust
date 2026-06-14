-- Seed: equipment profiles (BIAB 19L, 3-vessel 23L, 5hL commercial). Idempotent via ON CONFLICT (id) DO NOTHING.

INSERT INTO equipment_profiles (id, tenant_id, name, batch_size_liters, boil_size_liters, boil_time_minutes, boil_off_rate_liters_per_hour, mash_efficiency_pct, brewhouse_efficiency_pct, trub_loss_liters, grain_absorption_l_per_kg, notes)
VALUES
    (
        '33333333-3333-3333-3333-000000000001',
        '00000000-0000-0000-0000-000000000000',
        'Homebrew BIAB 19 L',
        19.0, 25.0, 60, 3.5, 72.0, 68.0, 1.5, 0.8,
        'Typical 19 L brew-in-a-bag setup. Single vessel. High efficiency achievable with a good squeeze.'
    ),
    (
        '33333333-3333-3333-3333-000000000002',
        '00000000-0000-0000-0000-000000000000',
        'Homebrew 3-Vessel 23 L',
        23.0, 29.0, 60, 4.0, 75.0, 72.0, 2.0, 1.0,
        'Typical 23 L 3-vessel HERMS or RIMS setup. Separate mash tun, HLT, and kettle.'
    ),
    (
        '33333333-3333-3333-3333-000000000003',
        '00000000-0000-0000-0000-000000000000',
        'Microbrewery 5 hL',
        500.0, 580.0, 90, 40.0, 80.0, 78.0, 15.0, 1.1,
        'Typical 5 hL (500 L) commercial cellar system. Conical fermenters, glycol chilling.'
    )
ON CONFLICT (id) DO NOTHING;

"""Build app/data/teams.json: Elo + expected goals for every World Cup team.

Slow-moving strength data, refreshed daily. The Rust engine reads the JSON;
the live match state (injuries, lineups, scores) comes from API-FOOTBALL at
runtime, not here.

Sources (all free):
  - National-team xG for/against: FBref via soccerdata (Opta-sourced).
  - National-team Elo: bundled CSV from eloratings.net / Kaggle export.

Usage:
  pip install -r requirements.txt
  python build_teams.py --elo-csv elo_2026.csv --out ../app/data/teams.json

If soccerdata cannot reach FBref (Cloudflare), the script falls back to the xG
already in teams.json so a refresh never wipes the bundle.
"""

import argparse
import json
import sys
from pathlib import Path

# 48 qualified teams keyed to their FBref World Cup spelling.
WORLD_CUP_TEAMS = [
    "Spain", "France", "Argentina", "Brazil", "England", "Portugal",
    "Netherlands", "Germany", "Italy", "Croatia", "Belgium", "Uruguay",
    "Morocco", "United States", "Mexico", "Canada",
    # ... extend to the full 48 once the draw is final.
]

ALIASES = {
    "United States": ["USA", "USMNT"],
    "Netherlands": ["NED", "Holland"],
}


def load_elo(elo_csv: str) -> dict:
    """Read national-team Elo from a CSV with columns: team,elo."""
    import csv
    ratings = {}
    with open(elo_csv, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            ratings[row["team"].strip()] = float(row["elo"])
    return ratings


def fetch_xg() -> dict:
    """Per-team xG for/against from FBref World Cup data via soccerdata."""
    try:
        import soccerdata as sd
    except ImportError:
        print("soccerdata not installed; skipping xG fetch", file=sys.stderr)
        return {}

    try:
        fbref = sd.FBref(leagues="INT-World Cup", seasons="2026")
        stats = fbref.read_team_season_stats(stat_type="standard")
    except Exception as e:  # Cloudflare / no data yet
        print(f"FBref fetch failed ({e}); keeping existing xG", file=sys.stderr)
        return {}

    xg = {}
    for team, row in stats.iterrows():
        name = team[-1] if isinstance(team, tuple) else str(team)
        games = max(float(row.get(("Playing Time", "MP"), 1)) or 1, 1)
        xg[name] = {
            "xg_for": round(float(row.get(("Expected", "xG"), 0)) / games, 2),
            "xg_against": round(float(row.get(("Expected", "xGA"), 0)) / games, 2),
        }
    return xg


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--elo-csv", required=False, help="CSV with team,elo columns")
    ap.add_argument("--out", default="../app/data/teams.json")
    args = ap.parse_args()

    out_path = Path(args.out)
    existing = {}
    if out_path.exists():
        for row in json.loads(out_path.read_text()):
            existing[row["name"]] = row

    elo = load_elo(args.elo_csv) if args.elo_csv else {}
    xg = fetch_xg()

    rows = []
    for team in WORLD_CUP_TEAMS:
        prev = existing.get(team, {})
        x = xg.get(team, {})
        rows.append({
            "name": team,
            "elo": elo.get(team, prev.get("elo", 1700.0)),
            "xg_for": x.get("xg_for", prev.get("xg_for", 1.3)),
            "xg_against": x.get("xg_against", prev.get("xg_against", 1.3)),
            "aliases": ALIASES.get(team, prev.get("aliases", [])),
        })

    out_path.write_text(json.dumps(rows, indent=2) + "\n")
    print(f"wrote {len(rows)} teams to {out_path}")


if __name__ == "__main__":
    main()

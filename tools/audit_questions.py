#!/usr/bin/env python3
"""Audits Bit's World Check explanations with TypeSafe.

`cargo test` proves every marked answer is right, but it can't tell whether
the explanation underneath makes sense. This asks TypeSafe a few questions
about each explanation and writes a report of the ones a person should read:

  - position:  does it point at a choice by where it's shown ("the middle
               program", "square B")? Choices are shuffled, so that can end
               up describing the wrong one. This is what the keyword test
               `explanations_never_point_at_a_position` approximates.
  - agreement: does it support the right answer, contradict it, or not say?
               (Skipped for map-square questions: matching a described walk
               to grid coordinates isn't a quick judgment.)
  - reading:   is it easy reading for an 8-12 year old?
  - reasoning: does it say why, or only restate the answer?
  - wrong:     does it say why a wrong choice fails? (Shown, never flagged.)

Usage, from the repo root:

    cargo test export_questions -- --ignored
    TYPESAFE_API_KEY=... python3 tools/audit_questions.py

Answers are cached in target/question-audit/cache.json, keyed by everything
sent, so re-running only asks about explanations or questions that changed.
Thresholds live in `decide()`; changing them needs no new requests. Exits 1
if any explanation fails, so it can gate a release. Standard library only.
"""

import argparse
import concurrent.futures
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

API = "https://api.typesafe.ai/v1/systemone"
MODEL = "jev-latest"
OUT = Path(__file__).resolve().parent.parent / "target" / "question-audit"

CONTEXT = (
    "A multiple-choice quiz question from Block Coders, a coding game for kids "
    "aged 8-12. A miner on a grid map follows a program made of blocks "
    "(FORWARD, TURN LEFT, TURN RIGHT, MINE, PLACE, REPEAT, IF, WHILE, functions). "
    "Programs are shown here as the equivalent Rust code. After the kid answers, "
    "the robot guide Bit shows `explanation`. The choices are shuffled into a "
    "random order before they are shown, and labelled A, B, C, D in that "
    "shuffled order."
)

MAP_KEY = (
    "Each string is one row, top row first; columns and rows count from 0 at the "
    "top left. . floor, # stone, C cobblestone, D diamond ore, T tree, B bedrock, "
    "W water, L lava, E emerald, P portal; ^ > v < the miner facing up, right, "
    "down, left. Squares that change every run: ? stone or empty, ! lava or "
    "empty, * stone or lava, $ diamond or empty, ~ water or empty, & stone, lava "
    "or empty, p maybe the portal."
)


def state_for(q):
    s = {"context": CONTEXT, "question": q["prompt"]}
    if q["program"]:
        s["program"] = q["program"]
    if q["map"]:
        s["map"] = {"rows": q["map"], "key": MAP_KEY}
    if q["blocks"]:
        s["blocks_in_backpack_at_start"] = q["blocks"]
    s["right_answer"] = q["choices"][0]
    s["wrong_answers"] = q["choices"][1:]
    s["explanation"] = q["explain"]
    return s


QUESTIONS = {
    "position": {
        "type": "noul",
        "instructions": (
            "Does `explanation` refer to one of the answer choices by its place "
            "on screen or its letter, instead of by what it says or does?"
        ),
        "criteria": {
            "true": (
                "Names a choice by order, position or label: the first/second/last "
                "one, the middle program, the one on the left, the top answer, "
                "option B, square C, the other one."
            ),
            "false": (
                "Only describes what programs do or what happens in the game. "
                "Directions the miner moves or faces (turn right, the square above "
                "the miner, the left side of the map) and counting steps or turns "
                "are about the game world, not about where a choice is shown."
            ),
        },
    },
    "agreement": {
        "type": "choice",
        "instructions": "How does `explanation` relate to `right_answer`?",
        "criteria": {
            "supports": "It explains why `right_answer` is right, consistent with what the program does.",
            "contradicts": (
                "It describes an outcome, count or program that conflicts with "
                "`right_answer`, or argues for one of `wrong_answers`."
            ),
            "says_nothing": "It neither supports nor contradicts `right_answer`.",
        },
    },
    "reading": {
        "type": "score",
        "instructions": "How easy is `explanation` for a 10-year-old to read and follow?",
        "criteria": [
            "Hard: long or tangled sentences, or abstract or technical words that are never explained.",
            "Readable with effort: mostly plain words, but one long sentence or one unexplained term.",
            "Easy: short sentences and everyday words. Game block names like FORWARD, REPEAT or WHILE count as everyday here.",
        ],
    },
    "reasoning": {
        "type": "noul",
        "instructions": (
            "Does `explanation` say why `right_answer` is right, for example by "
            "walking through what the program does or naming the idea that decides "
            "it, rather than only restating the answer?"
        ),
    },
    "wrong": {
        "type": "noul",
        "instructions": (
            "Does `explanation` say why at least one of `wrong_answers` fails or "
            "what mistake would lead a kid to pick it?"
        ),
    },
}


def questions_for(q):
    if q["kind"] == "spots":
        return {k: v for k, v in QUESTIONS.items() if k != "agreement"}
    return QUESTIONS


def cache_key(body):
    return hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()


def ask(body, key):
    req = urllib.request.Request(
        API,
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
    )
    for attempt in range(5):
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            if e.code != 429 and e.code < 500 or attempt == 4:
                detail = e.read().decode(errors="replace")[:500]
                raise RuntimeError(f"HTTP {e.code}: {detail}") from None
        except urllib.error.URLError:
            if attempt == 4:
                raise
        time.sleep(2**attempt)


# What counts as a problem. Tune these against explanations you've read.
POSITION_FAIL = 0.7
POSITION_REVIEW = 0.3
AGREEMENT_SURE = 0.8
READING_REVIEW = 1.0
REASONING_REVIEW = 0.3


def decide(q, a):
    """Turns raw answers into (status, reasons). No API calls."""
    fails, reviews = [], []
    p = a["position"]["noul"]
    if p >= POSITION_FAIL:
        fails.append(f"points at a choice by position ({p:.2f})")
    elif p >= POSITION_REVIEW:
        reviews.append(f"might point at a choice by position ({p:.2f})")
    if "agreement" in a:
        ag = a["agreement"]
        if ag["choice"] == "contradicts":
            (fails if ag["confidence"] >= AGREEMENT_SURE else reviews).append(
                f"contradicts the right answer (confidence {ag['confidence']:.2f})"
            )
        elif ag["choice"] == "says_nothing":
            reviews.append(f"doesn't support the right answer (confidence {ag['confidence']:.2f})")
    r = a["reading"]["score"]
    if r < READING_REVIEW:
        reviews.append(f"hard reading for 8-12 (score {r:.2f} of 2)")
    w = a["reasoning"]["noul"]
    if w < REASONING_REVIEW:
        reviews.append(f"restates the answer without saying why ({w:.2f})")
    return ("FAIL" if fails else "REVIEW" if reviews else "OK"), fails + reviews


def report(rows, model):
    lines = ["# World Check explanation audit", "", f"Model: `{model}`", ""]
    counts = {s: sum(1 for r in rows if r["status"] == s) for s in ("FAIL", "REVIEW", "OK")}
    lines.append(f"{counts['FAIL']} fail, {counts['REVIEW']} to review, {counts['OK']} OK, of {len(rows)}.")
    for status in ("FAIL", "REVIEW"):
        flagged = [r for r in rows if r["status"] == status]
        if not flagged:
            continue
        lines += ["", f"## {status}", ""]
        for r in flagged:
            q = r["q"]
            lines.append(f"### {q['id']}: {q['prompt']}")
            lines.append("")
            lines += [f"- {why}" for why in r["reasons"]]
            lines += ["", f"> {q['explain']}", ""]
    lines += ["", "## All answers", "", "| id | status | position | agreement | reading | reasoning | wrong |", "|---|---|---|---|---|---|---|"]
    for r in rows:
        a = r["a"]
        ag = f"{a['agreement']['choice']} {a['agreement']['confidence']:.2f}" if "agreement" in a else "-"
        lines.append(
            f"| {r['q']['id']} | {r['status']} | {a['position']['noul']:.2f} | {ag} | "
            f"{a['reading']['score']:.2f} | {a['reasoning']['noul']:.2f} | {a['wrong']['noul']:.2f} |"
        )
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--only", nargs="*", help="question ids to audit (default: all)")
    ap.add_argument("--dry-run", action="store_true", help="print the first request and exit")
    args = ap.parse_args()

    src = OUT / "questions.json"
    if not src.exists():
        sys.exit(f"{src} not found. Run: cargo test export_questions -- --ignored")
    qs = json.loads(src.read_text())
    if args.only:
        qs = [q for q in qs if q["id"] in args.only]
    bodies = [{"model": MODEL, "state": state_for(q), "questions": questions_for(q)} for q in qs]
    if args.dry_run:
        print(json.dumps(bodies[0], indent=2))
        return

    cache_path = OUT / "cache.json"
    cache = json.loads(cache_path.read_text()) if cache_path.exists() else {}
    todo = [b for b in bodies if cache_key(b) not in cache]
    if todo:
        key = os.environ.get("TYPESAFE_API_KEY")
        if not key:
            sys.exit("Set TYPESAFE_API_KEY (from the TypeSafe console).")
        print(f"Asking about {len(todo)} of {len(bodies)} explanations ({len(bodies) - len(todo)} cached)...", file=sys.stderr)
        tokens = 0
        with concurrent.futures.ThreadPoolExecutor(8) as pool:
            futures = {pool.submit(ask, b, key): b for b in todo}
            for f in concurrent.futures.as_completed(futures):
                resp = f.result()
                cache[cache_key(futures[f])] = resp
                tokens += resp["usage"]["input_tokens"]
        cache_path.write_text(json.dumps(cache))
        print(f"Used {tokens} input tokens.", file=sys.stderr)

    rows = []
    for q, b in zip(qs, bodies):
        resp = cache[cache_key(b)]
        status, reasons = decide(q, resp["answers"])
        rows.append({"q": q, "a": resp["answers"], "status": status, "reasons": reasons, "model": resp["model"]})
    model = ", ".join(sorted({r["model"] for r in rows})) or MODEL
    (OUT / "report.md").write_text(report(rows, model))

    for r in rows:
        if r["status"] != "OK":
            print(f"{r['status']:6} {r['q']['id']}: {'; '.join(r['reasons'])}")
    fails = sum(r["status"] == "FAIL" for r in rows)
    print(f"\nReport: {OUT / 'report.md'}")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()

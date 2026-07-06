#!/usr/bin/env python3
"""Regenerate the vendored domain-classification data files in this directory.

Curation-time only — the server never fetches anything at runtime; it embeds
the generated files via include_str!. Re-run this script to refresh the data,
review the diff (especially the conflict/frequency report on stderr), and
commit the result.

    python3 backend/assets/domain-classification/refresh-data.py

Outputs (all in this directory):
  freemail-domains.txt      one domain per line; consumer/ISP mailbox providers
  university-domains.txt    one domain per line; world universities
  institutional-domains.csv "domain,type" map for the institutional residual

Sources (all open-licensed, fetched over HTTPS):
  - Kikobeats/free-email-domains (MIT)       -> freemail
  - Hipo/university-domains-list (MIT)       -> education
  - Wikidata SPARQL, P856 official website (CC0):
      hospitals Q16917                        -> healthcare
      public utilities Q1951366               -> utility
      public transport companies Q2127330     -> utility
      municipalities Q15284                   -> government
  - Annuaire de l'administration, opendatasoft mirror of the DILA base
    (Licence Ouverte v2) — French local administrations incl. DOM-COM.
    URL field only; the email field often carries ISP mailbox domains.
                                              -> government
    (Authoritative-but-heavy alternative: DILA all_latest.tar.bz2, ~360 MB,
    https://lecomarquage.service-public.gouv.fr/donnees_locales_v4/all_latest.tar.bz2)
  - GSA/govt-urls (US public domain) — US government sites, including the
    ones NOT under .gov/.mil                  -> government

institutional-overrides.json is hand-maintained and NOT touched by this script.
"""

import csv
import io
import json
import re
import sys
import time
import urllib.parse
import urllib.request
from collections import Counter
from datetime import date
from pathlib import Path

OUT_DIR = Path(__file__).parent
# Raw downloads are cached here so an interrupted run (e.g. Wikidata's 1
# req/min outage throttling) resumes instead of refetching. Delete the
# directory (or single files) to force a refetch.
CACHE_DIR = OUT_DIR / ".cache"
USER_AGENT = "ScanopyDomainClassification/1.0 (dataset curation script)"
WIKIDATA_SPARQL = "https://query.wikidata.org/sparql"

# Mailbox/ISP providers seen in real signups that are missing from (or worth
# pinning independently of) the Kikobeats list.
CURATED_FREEMAIL = """
gmail.com googlemail.com yahoo.com yahoo.co.uk yahoo.co.jp yahoo.fr yahoo.de
hotmail.com outlook.com live.com msn.com icloud.com me.com mac.com proton.me
protonmail.com pm.me gmx.com gmx.de gmx.net web.de aol.com mail.com mail.ru
yandex.ru yandex.com zoho.com fastmail.com hey.com t-online.de freenet.de
arcor.de orange.fr wanadoo.fr free.fr sfr.fr laposte.net neuf.fr bbox.fr
libero.it tiscali.it seznam.cz centrum.cz qq.com 163.com 126.com naver.com
daum.net duck.com duckduckgo.com tutanota.com tuta.io tuta.com mailbox.org
posteo.de posteo.net mailfence.com hushmail.com bluewin.ch sunrise.ch
telenet.be skynet.be ziggo.nl xs4all.nl home.nl planet.nl comcast.net
verizon.net att.net sbcglobal.net cox.net charter.net earthlink.net bell.net
sympatico.ca rogers.com shaw.ca telus.net bigpond.com optusnet.com.au
iinet.net.au tpg.com.au btinternet.com virginmedia.com talktalk.net
ntlworld.com sky.com o2.co.uk bellsouth.net juno.com ocn.ne.jp biglobe.ne.jp
nifty.com so-net.ne.jp mail.pf simplelogin.com simplelogin.io slmail.me
anonaddy.me mozmail.com
""".split()

# Hosting platforms, social networks and blog services that institutions list
# as their "official website". An email domain equal to one of these proves
# nothing about the sender's organization — drop the entry.
DENY_SUFFIXES = """
facebook.com wixsite.com wix.com wordpress.com blogspot.com blogspot.fr
blogspot.de blogspot.co.uk google.com sites.google.com weebly.com jimdo.com
jimdofree.com jimdosite.com business.site notion.site github.io gitlab.io
over-blog.com over-blog.fr canalblog.com e-monsite.com pagesperso-orange.fr
monsite-orange.fr wifeo.com instagram.com twitter.com x.com youtube.com
linktr.ee archive.org tripod.com angelfire.com webs.com yolasite.com
webnode.com webnode.fr webself.net site123.me godaddysites.com squarespace.com
carrd.co netlify.app vercel.app herokuapp.com sharepoint.com wordpress.org
tumblr.com medium.com
ghidulprimariilor.ro inforpressca.com lapagelocale.fr e-primarii.ro
""".split()

INSTITUTIONAL_SOURCES = [
    # (label, type, fetch function name) — precedence on conflicts follows
    # TYPE_PRECEDENCE below, then first-listed source wins within a type.
    ("wikidata hospitals (Q16917)", "healthcare", "fetch_wikidata", "Q16917"),
    ("wikidata public utilities (Q1951366)", "utility", "fetch_wikidata", "Q1951366"),
    ("wikidata transport companies (Q2127330)", "utility", "fetch_wikidata", "Q2127330"),
    ("wikidata municipalities (Q15284)", "government", "fetch_wikidata", "Q15284"),
    ("annuaire de l'administration (FR)", "government", "fetch_annuaire", None),
    ("GSA govt-urls (US)", "government", "fetch_gsa", None),
]

# More specific wins: a commune-run hospital should read healthcare.
TYPE_PRECEDENCE = {"healthcare": 0, "utility": 1, "government": 2}


def fetch(url: str, timeout: int = 180) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return resp.read()


def cached(name: str, produce) -> str:
    path = CACHE_DIR / name
    if path.exists():
        print(f"cache hit: {name}", file=sys.stderr)
        return path.read_text()
    text = produce()
    CACHE_DIR.mkdir(exist_ok=True)
    path.write_text(text)
    return text


def normalize_host(raw: str) -> str | None:
    """URL or bare host -> lowercase punycode host without www., or None."""
    raw = raw.strip().strip('"').lower()
    if not raw or " " in raw:
        return None
    if "//" not in raw:
        raw = "http://" + raw
    host = urllib.parse.urlparse(raw).hostname or ""
    host = host.strip(".")
    if host.startswith("www."):
        host = host[4:]
    if "." not in host or re.fullmatch(r"[0-9.]+", host):
        return None
    try:
        host = host.encode("idna").decode("ascii")
    except UnicodeError:
        return None
    if not re.fullmatch(r"[a-z0-9.-]+", host):
        return None
    return host


def denied(host: str) -> bool:
    return any(host == d or host.endswith("." + d) for d in DENY_SUFFIXES)


def fetch_wikidata(qid: str) -> list[str]:
    def produce() -> str:
        query = (
            "SELECT DISTINCT ?site WHERE { "
            f"?item wdt:P31/wdt:P279* wd:{qid} . ?item wdt:P856 ?site }}"
        )
        url = WIKIDATA_SPARQL + "?" + urllib.parse.urlencode({"query": query})
        req = urllib.request.Request(
            url, headers={"User-Agent": USER_AGENT, "Accept": "text/csv"}
        )
        for attempt in range(8):
            try:
                with urllib.request.urlopen(req, timeout=180) as resp:
                    text = resp.read().decode("utf-8")
                time.sleep(2)  # be polite between queries
                return text
            except urllib.error.HTTPError as e:
                if e.code != 429 or attempt == 7:
                    raise
                print(f"wikidata {qid}: 429, waiting 70s", file=sys.stderr)
                time.sleep(70)
        raise AssertionError("unreachable")

    return cached(f"wikidata-{qid}.csv", produce).splitlines()[1:]  # skip header


def fetch_annuaire(_arg) -> list[str]:
    url = (
        "https://public.opendatasoft.com/api/explore/v2.1/catalog/datasets/"
        "annuaire-de-ladministration-base-de-donnees-locales/exports/csv"
        "?select=coordonneesnum_url&delimiter=%3B"
    )
    text = cached(
        "annuaire.csv", lambda: fetch(url, timeout=300).decode("utf-8", errors="replace")
    )
    text = text.lstrip("\ufeff")  # export ships a UTF-8 BOM
    reader = csv.DictReader(io.StringIO(text), delimiter=";")
    return [row["coordonneesnum_url"] for row in reader if row.get("coordonneesnum_url")]


def fetch_gsa(_arg) -> list[str]:
    text = cached(
        "gsa.csv",
        lambda: fetch(
            "https://raw.githubusercontent.com/GSA/govt-urls/master/1_govt_urls_full.csv"
        ).decode("utf-8", errors="replace"),
    )
    reader = csv.DictReader(io.StringIO(text))
    col = next((c for c in reader.fieldnames or [] if "domain" in c.lower() or "url" in c.lower()), None)
    if col is None:
        raise RuntimeError(f"GSA govt-urls: no domain column in {reader.fieldnames}")
    return [row[col] for row in reader if row.get(col)]


def main() -> None:
    today = date.today().isoformat()

    # --- freemail ---
    kikobeats = json.loads(
        fetch("https://raw.githubusercontent.com/Kikobeats/free-email-domains/master/domains.json")
    )
    freemail = {normalize_host(d) for d in kikobeats + CURATED_FREEMAIL}
    freemail.discard(None)
    header = (
        f"# Generated by refresh-data.py on {today}. Do not edit by hand.\n"
        "# Sources: Kikobeats/free-email-domains (MIT) + curated ISP/alias additions.\n"
    )
    (OUT_DIR / "freemail-domains.txt").write_text(
        header + "\n".join(sorted(freemail)) + "\n"
    )
    print(f"freemail-domains.txt: {len(freemail)} domains", file=sys.stderr)

    # --- universities ---
    unis = json.loads(
        fetch(
            "https://raw.githubusercontent.com/Hipo/university-domains-list/master/"
            "world_universities_and_domains.json"
        )
    )
    uni_domains = {normalize_host(d) for u in unis for d in u.get("domains", [])}
    uni_domains.discard(None)
    uni_domains -= freemail
    header = (
        f"# Generated by refresh-data.py on {today}. Do not edit by hand.\n"
        "# Source: Hipo/university-domains-list (MIT).\n"
    )
    (OUT_DIR / "university-domains.txt").write_text(
        header + "\n".join(sorted(uni_domains)) + "\n"
    )
    print(f"university-domains.txt: {len(uni_domains)} domains", file=sys.stderr)

    # --- institutional residual ---
    chosen: dict[str, str] = {}
    frequency: Counter[str] = Counter()
    for label, inst_type, fn_name, arg in INSTITUTIONAL_SOURCES:
        raw = globals()[fn_name](arg)
        normalized = [h for h in (normalize_host(r) for r in raw) if h]
        frequency.update(normalized)  # pre-dedup: platforms recur across records
        hosts = set(normalized)
        kept = 0
        for host in hosts:
            if denied(host) or host in freemail or host in uni_domains:
                continue
            kept += 1
            prev = chosen.get(host)
            if prev is None:
                chosen[host] = inst_type
            elif prev != inst_type:
                if TYPE_PRECEDENCE[inst_type] < TYPE_PRECEDENCE[prev]:
                    print(f"conflict: {host} {prev} -> {inst_type}", file=sys.stderr)
                    chosen[host] = inst_type
        print(f"{label}: {len(hosts)} hosts, {kept} kept", file=sys.stderr)

    header = (
        f"# Generated by refresh-data.py on {today}. Do not edit by hand\n"
        "# (hand corrections belong in institutional-overrides.json).\n"
        "# domain,type where type is government|education|healthcare|utility.\n"
        "# Sources: Wikidata P856 (CC0), Annuaire de l'administration (Licence\n"
        "# Ouverte v2), GSA/govt-urls (US public domain).\n"
    )
    lines = [f"{d},{t}" for d, t in sorted(chosen.items())]
    (OUT_DIR / "institutional-domains.csv").write_text(header + "\n".join(lines) + "\n")
    print(f"institutional-domains.csv: {len(chosen)} domains", file=sys.stderr)

    # Hosts shared by many records are usually platforms, not org domains —
    # review candidates for DENY_SUFFIXES.
    print("most shared hosts (deny-list review):", file=sys.stderr)
    for host, n in frequency.most_common(15):
        print(f"  {n:>4}  {host}", file=sys.stderr)


if __name__ == "__main__":
    main()

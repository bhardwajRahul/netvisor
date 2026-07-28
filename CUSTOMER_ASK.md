# Customer ask — slow L2 Physical view

Draft for the maintainer to send. Everything below is optional for them; each
item independently sharpens the fix, and item 5 alone would pinpoint the
bottleneck outright.

---

Hi — thanks for flagging the slow **L2 Physical** view. We're actively working
on rendering performance at that scale, and we've built a synthetic test graph
locally to profile against. To make sure we're targeting *your* bottleneck
rather than a generic guess, a few things would help enormously:

**1. How big is the view, roughly?**
Approximate counts are fine — the Hosts / Inventory pages will give you these at
a glance:

- how many **hosts** appear in the L2 Physical view
- roughly how many **interfaces / ports** per host
- how many **physical links** between them

Order of magnitude is what matters; we don't need exact numbers.

**2. What exactly is slow, and when?**
These are different problems with different fixes, so it's worth separating:

- the **initial load** — the delay before the diagram first appears
- **panning / zooming / clicking** afterwards, once it's drawn

And does it settle down once loaded, or stay sluggish the whole time you're on
the page?

**3. Your environment.**
This view renders entirely in the browser, so client specs matter a lot more
than server specs here:

- browser and version
- rough machine specs (CPU, RAM), and whether it's a laptop on battery

**4. A reproduction, ideally anonymised.**
If you can send an export that lets us rebuild your graph shape locally, we can
profile against the real thing. The **Hosts export** in the app should be
enough, provided it captures interfaces and their physical links — a previous
customer sent us a `hosts-export.zip` and that worked well.

Please feel free to **redact hostnames and IPs** — we only need the *structure
and the counts* (how many hosts, how they're wired together), not what anything
is called.

**5. Best of all, if it's easy: a browser performance trace.**
This one pinpoints the bottleneck directly, and takes about a minute:

1. Open the L2 Physical view's page but don't load the view yet.
2. Press **F12** to open developer tools, and pick the **Performance** tab.
3. Click **Record** (the circle), then load / interact with the slow view.
4. Stop recording after ~10–20 seconds of the slowness.
5. Click **Save profile** (the download arrow) and send us the `.json` file.

Traces can be large; a zip or file-share link is fine. They contain page timing
data rather than your infrastructure details.

No pressure on any of these — even just answers to (1) and (2) would meaningfully
narrow things down.

---

## Notes for the maintainer (not part of the message)

- **Why (2) matters most after the counts:** initial load and interaction are
  separate bottlenecks in this codebase. Slow *first paint* points at the
  measure pass and ELK layout; slow *pan/zoom* points at the sheer number of
  mounted DOM nodes, which viewport culling addresses. The fixes differ, so the
  answer changes what we prioritise.
- **Why (1) matters:** in L2 Physical each **host is a container** and each
  **interface is an element**, so "300 hosts" can mean several thousand rendered
  nodes. Interfaces-per-host is therefore as important as the host count, and
  it's the number people don't think to give.
- **On (4):** confirm the hosts export actually includes interface + neighbour
  data before promising it's sufficient — if it doesn't, the structure counts
  from (1) are enough to rebuild an equivalent graph with
  `backend/scripts/seed-l2-perf.sql`.

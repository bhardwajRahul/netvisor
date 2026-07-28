# Customer ask — slow L2 Physical view

Draft for the maintainer to send.

---

Hi — following up on the slow **L2 Physical** view. We've made some significant
rendering improvements: the view loads noticeably faster at scale, and panning
and zooming should now be smooth rather than stuttery.

Could you try it once you're on the new build and let us know whether it's
sorted for you?

If it's still slow, one thing would help us a lot: roughly **how many hosts**
are in that view. We tuned against a synthetic estate of about 400 — if yours
is much larger, that alone would explain it.

---

## Notes for the maintainer (not part of the message)

**Why this is so short.** The earlier draft asked for host/interface/link
counts, environment details, an anonymised data export and a DevTools
performance trace. That is a lot of homework for someone whose only obligation
was to tell us it was slow — and most of it was written *before* the fixes
landed, when we genuinely didn't know where the time went. We do now, so the
only question that still earns its place is "did this fix it".

**Escalation path, if they say it's still slow.** Ask for these *then*, in this
order — not upfront, and not all at once:

1. **Interfaces per host.** The number people never think to give, and the one
   that matters most: in L2 each host is a container and each interface an
   element, so 300 hosts can mean several thousand rendered nodes. Host count
   alone can understate the graph by an order of magnitude.
2. **Which part is still slow** — the initial load, or interaction afterwards.
   Different causes, different fixes.
3. **Browser and rough machine specs.** It renders entirely client-side, so this
   matters more than server specs.
4. **A DevTools performance trace.** Pinpoints the blocker directly, but it is
   the biggest ask — only worth requesting if the cheaper answers don't explain
   it.

**On requesting a data export.** Don't promise the Hosts export is sufficient
without checking it first — an earlier draft did, and nobody has confirmed it
captures interfaces and their neighbour links. It probably isn't needed anyway:
given counts from (1), `backend/scripts/seed-l2-perf.sql` rebuilds an equivalent
graph locally without the customer sending anything.

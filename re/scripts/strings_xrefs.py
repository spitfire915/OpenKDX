# Prints every defined string containing one of the needles and the functions
# that reference it. Output lines start with "@@ " so re/ghidra-run.sh can
# pick them out of Ghidra's log.
#
#   re/ghidra-run.sh KDXServer.lexe strings_xrefs.py Accounts.dat ".stg"
# @category OpenKDX
from ghidra.program.model.symbol import RefType

needles = list(getScriptArgs())
listing = currentProgram.getListing()
fm = currentProgram.getFunctionManager()
refs = currentProgram.getReferenceManager()


def out(s):
    println("@@ " + s)


for data in listing.getDefinedData(True):
    if not data.hasStringValue():
        continue
    try:
        value = str(data.getValue())
    except Exception:
        continue
    if needles and not any(n in value for n in needles):
        continue
    funcs = {}
    for r in refs.getReferencesTo(data.getMinAddress()):
        f = fm.getFunctionContaining(r.getFromAddress())
        funcs.setdefault(f.getName() if f else "?", []).append(str(r.getFromAddress()))
        # Pointers to the string stored in data (vtables, tables).
        for r2 in refs.getReferencesTo(r.getFromAddress()):
            f2 = fm.getFunctionContaining(r2.getFromAddress())
            if f2:
                funcs.setdefault(f2.getName(), []).append(str(r2.getFromAddress()))
    shown = value.replace("\n", "\\n").replace("\t", "\\t")[:80]
    where = "; ".join("%s@%s" % (f, ",".join(sorted(set(a)))) for f, a in sorted(funcs.items()))
    out("%s\t%r\t%s" % (data.getMinAddress(), shown, where))

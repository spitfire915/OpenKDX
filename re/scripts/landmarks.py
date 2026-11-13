# Prints "FUN_… entry size callers callees" for the functions named as
# arguments, for pasting into re/landmarks.md. With no arguments, lists the
# functions that call the libc file-writing functions (fwrite, write) and
# whose string references mention a file name: the on-disk writers.
#
#   re/ghidra-run.sh KDXServer.lexe landmarks.py FUN_08060268
# @category OpenKDX
from ghidra.util.task import ConsoleTaskMonitor

fm = currentProgram.getFunctionManager()
monitor_ = ConsoleTaskMonitor()


def out(s):
    println("@@ " + s)


def describe(f):
    callers = sorted(c.getName() for c in f.getCallingFunctions(monitor_))
    callees = sorted(c.getName() for c in f.getCalledFunctions(monitor_))
    out("%s\t%s\t%d\tcallers=%s\tcallees=%s" % (
        f.getName(), f.getEntryPoint(), f.getBody().getNumAddresses(),
        ",".join(callers)[:200], ",".join(callees)[:200]))


names = list(getScriptArgs())
if names:
    for f in fm.getFunctions(True):
        if f.getName() in names:
            describe(f)
else:
    sinks = ("fwrite", "write", "fputs", "fprintf", "fopen", "open", "rename")
    for f in fm.getFunctions(True):
        callees = set(c.getName() for c in f.getCalledFunctions(monitor_))
        if callees & set(sinks):
            describe(f)

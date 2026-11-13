// Prints "name entry size callers callees" for the functions named as
// arguments, for pasting into re/landmarks.md. With no arguments, lists the
// functions that call a libc file-writing routine: the on-disk writers.
//
//   re/ghidra-run.sh KDXServer.lexe Landmarks.java FUN_08060268
// @category OpenKDX
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import java.util.*;

public class Landmarks extends GhidraScript {
    private static final Set<String> SINKS =
        new HashSet<>(Arrays.asList("fwrite", "write", "fputs", "fprintf", "fopen", "open", "rename"));

    @Override
    public void run() throws Exception {
        Set<String> names = new HashSet<>(Arrays.asList(getScriptArgs()));
        FunctionManager fm = currentProgram.getFunctionManager();
        for (Function f : fm.getFunctions(true)) {
            if (!names.isEmpty()) {
                if (names.contains(f.getName())) describe(f);
                continue;
            }
            for (Function c : f.getCalledFunctions(monitor)) {
                if (SINKS.contains(c.getName())) { describe(f); break; }
            }
        }
    }

    private void describe(Function f) {
        TreeSet<String> callers = new TreeSet<>();
        for (Function c : f.getCallingFunctions(monitor)) callers.add(c.getName());
        TreeSet<String> callees = new TreeSet<>();
        for (Function c : f.getCalledFunctions(monitor)) callees.add(c.getName());
        println("@@ " + f.getName() + "\t" + f.getEntryPoint() + "\t" + f.getBody().getNumAddresses()
            + "\tcallers=" + cut(String.join(",", callers)) + "\tcallees=" + cut(String.join(",", callees)));
    }

    private static String cut(String s) { return s.length() > 300 ? s.substring(0, 300) + "…" : s; }
}

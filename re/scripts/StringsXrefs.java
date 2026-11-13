// Prints every defined string containing one of the needles and the functions
// that reference it. Output lines start with "@@ " so re/ghidra-run.sh can
// pick them out of Ghidra's log.
//
//   re/ghidra-run.sh KDXServer.lexe StringsXrefs.java Accounts.dat ".stg"
// @category OpenKDX
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceManager;
import java.util.*;

public class StringsXrefs extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] needles = getScriptArgs();
        FunctionManager fm = currentProgram.getFunctionManager();
        ReferenceManager refs = currentProgram.getReferenceManager();
        for (Data data : currentProgram.getListing().getDefinedData(true)) {
            if (!data.hasStringValue()) continue;
            Object v = data.getValue();
            if (v == null) continue;
            String value = v.toString();
            boolean hit = needles.length == 0;
            for (String n : needles) if (value.contains(n)) hit = true;
            if (!hit) continue;
            TreeMap<String, TreeSet<String>> funcs = new TreeMap<>();
            // Stripped i386 binaries often carry the string's address as a
            // bare immediate that analysis did not turn into a reference:
            // scan for it too.
            long target = data.getMinAddress().getOffset();
            byte[] le = { (byte) target, (byte) (target >> 8), (byte) (target >> 16), (byte) (target >> 24) };
            Address at = currentProgram.getMinAddress();
            while (at != null) {
                at = currentProgram.getMemory().findBytes(at, le, null, true, monitor);
                if (at == null) break;
                add(funcs, fm, at);
                at = at.add(1);
            }
            for (Reference r : refs.getReferencesTo(data.getMinAddress())) {
                add(funcs, fm, r.getFromAddress());
                // Pointers to the string stored in data (tables, vtables).
                for (Reference r2 : refs.getReferencesTo(r.getFromAddress())) {
                    add(funcs, fm, r2.getFromAddress());
                }
            }
            String shown = value.replace("\n", "\\n").replace("\t", "\\t");
            if (shown.length() > 80) shown = shown.substring(0, 80);
            StringBuilder where = new StringBuilder();
            for (Map.Entry<String, TreeSet<String>> e : funcs.entrySet()) {
                if (where.length() > 0) where.append("; ");
                where.append(e.getKey()).append("@").append(String.join(",", e.getValue()));
            }
            println("@@ " + data.getMinAddress() + "\t\"" + shown + "\"\t" + where);
        }
    }

    private void add(TreeMap<String, TreeSet<String>> funcs, FunctionManager fm, Address from) {
        Function f = fm.getFunctionContaining(from);
        String name = f == null ? "?" : f.getName();
        funcs.computeIfAbsent(name, k -> new TreeSet<>()).add(from.toString());
    }
}

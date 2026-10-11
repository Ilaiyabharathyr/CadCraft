package io.github.storytold.cadcraft;

import android.app.NativeActivity;
import android.app.AlertDialog;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.text.InputType;
import android.text.InputFilter;
import android.view.WindowManager;
import android.view.inputmethod.EditorInfo;
import android.widget.EditText;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.Locale;
import java.util.UUID;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Android lifecycle and document-provider glue. The drawing engine and UI run in Rust. */
public final class MainActivity extends NativeActivity {
    private static final int OPEN = 41;
    private static final int EXPORT = 42;
    private static final long MAX_BYTES = 256L * 1024 * 1024;
    private final ConcurrentLinkedQueue<String> imports = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<String> notices = new ConcurrentLinkedQueue<>();
    private final ConcurrentLinkedQueue<String> commands = new ConcurrentLinkedQueue<>();
    private final ExecutorService files = Executors.newSingleThreadExecutor();
    private boolean picking;
    private File exporting;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
    }

    /** Called by Rust; opening the provider and copying its result never blocks the drawing UI. */
    public void pickDrawing() {
        runOnUiThread(() -> {
            if (picking) return;
            picking = true;
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("*/*");
            try { startActivityForResult(intent, OPEN); }
            catch (RuntimeException error) { picking = false; notice("Open", error); }
        });
    }

    public void exportDrawing(String path, String name) {
        runOnUiThread(() -> {
            if (picking || isFinishing() || isDestroyed()) return;
            picking = true;
            files.execute(() -> {
                File snapshot = null;
                try {
                    File source = new File(path).getCanonicalFile();
                    String root = getFilesDir().getCanonicalPath() + File.separator;
                    if (!source.getPath().startsWith(root) || !source.isFile()) {
                        throw new IllegalArgumentException("The drawing has not been saved in the app");
                    }
                    // Copy off the UI thread. The picker exports this snapshot even if the
                    // drawing is edited or saved again while the picker is open.
                    snapshot = File.createTempFile("cadcraft-export-", ".drawing", getCacheDir());
                    try (InputStream in = new FileInputStream(source); OutputStream out = new FileOutputStream(snapshot)) {
                        copy(in, out);
                    }
                    File ready = snapshot;
                    runOnUiThread(() -> {
                        if (isFinishing() || isDestroyed()) { ready.delete(); picking = false; return; }
                        exporting = ready;
                        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
                        intent.addCategory(Intent.CATEGORY_OPENABLE);
                        intent.setType("application/octet-stream");
                        intent.putExtra(Intent.EXTRA_TITLE, name);
                        try { startActivityForResult(intent, EXPORT); }
                        catch (RuntimeException error) { picking = false; clearExport(); notice("Export", error); }
                    });
                } catch (Exception error) {
                    if (snapshot != null) snapshot.delete();
                    runOnUiThread(() -> { picking = false; notice("Export", error); });
                }
            });
        });
    }

    public String takeImportedPath() { return imports.poll(); }
    public String takeNotice() { return notices.poll(); }
    public String takeCommand() { return commands.poll(); }
    public String interfaceLocale() { return Locale.getDefault().toLanguageTag(); }

    /** NativeActivity has no composing-text support. Use a real Android edit field for commands,
     * coordinates and values, rather than relying on keyboard-specific synthetic key events. */
    public void enterCommand(String initial) {
        runOnUiThread(() -> {
            EditText input = new EditText(this);
            input.setSingleLine(true);
            input.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
            input.setImeOptions(EditorInfo.IME_ACTION_DONE);
            input.setFilters(new InputFilter[] { new InputFilter.LengthFilter(8192) });
            input.setText(initial);
            input.setSelection(input.length());
            int padding = (int) (20 * getResources().getDisplayMetrics().density);
            input.setPadding(padding, padding, padding, padding);
            AlertDialog dialog = new AlertDialog.Builder(this).setTitle("Command or value")
                .setView(input)
                .setPositiveButton("Enter", (d, which) -> commands.offer(input.getText().toString()))
                .setNegativeButton("Cancel", null).create();
            input.setOnEditorActionListener((view, action, event) -> {
                if (action != EditorInfo.IME_ACTION_DONE) return false;
                commands.offer(input.getText().toString());
                dialog.dismiss();
                return true;
            });
            dialog.setOnShowListener(d -> {
                input.requestFocus();
                if (dialog.getWindow() != null) dialog.getWindow().setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_VISIBLE);
            });
            dialog.show();
        });
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != OPEN && request != EXPORT) return;
        picking = false;
        if (result != RESULT_OK || data == null || data.getData() == null) {
            if (request == EXPORT) clearExport();
            return;
        }
        Uri uri = data.getData();
        File snapshot = exporting;
        exporting = null;
        files.execute(() -> {
            try {
                if (request == OPEN) importDrawing(uri);
                else if (snapshot != null) {
                    try (InputStream in = new FileInputStream(snapshot);
                         OutputStream out = getContentResolver().openOutputStream(uri, "wt")) {
                        if (out == null) throw new IllegalStateException("The file provider did not return a writable file");
                        copy(in, out);
                    }
                    notices.offer("Drawing exported");
                }
            } catch (Exception error) { notice(request == OPEN ? "Open" : "Export", error); }
            finally { if (snapshot != null) snapshot.delete(); }
        });
    }

    private void importDrawing(Uri uri) throws Exception {
        String name = "Drawing.dxf";
        try (Cursor cursor = getContentResolver().query(uri, null, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                int column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (column >= 0 && !cursor.isNull(column)) name = cursor.getString(column);
                column = cursor.getColumnIndex(OpenableColumns.SIZE);
                if (column >= 0 && !cursor.isNull(column) && cursor.getLong(column) > MAX_BYTES) {
                    throw new IllegalArgumentException("This drawing is larger than 256 MB");
                }
            }
        }
        name = name.replace('/', '_').replace('\\', '_').replaceAll("[\\p{Cntrl}]", "_");
        if (name.length() > 180) {
            String extension = name.toLowerCase(Locale.ROOT).endsWith(".dwg") ? ".dwg" : ".dxf";
            name = name.substring(0, 176) + extension;
        }
        String lower = name.toLowerCase(Locale.ROOT);
        if (!lower.endsWith(".dxf") && !lower.endsWith(".dwg")) {
            throw new IllegalArgumentException("Choose a DXF or DWG drawing");
        }
        File folder = new File(new File(getFilesDir(), "imports"), UUID.randomUUID().toString());
        if (!folder.mkdirs()) throw new IllegalStateException("Could not create the import folder");
        File drawing = new File(folder, name);
        try (InputStream in = getContentResolver().openInputStream(uri);
             OutputStream out = new FileOutputStream(drawing)) {
            if (in == null) throw new IllegalStateException("The file provider did not return a readable file");
            copy(in, out);
        } catch (Exception error) { drawing.delete(); folder.delete(); throw error; }
        imports.offer(drawing.getAbsolutePath());
    }

    private static void copy(InputStream in, OutputStream out) throws Exception {
        byte[] buffer = new byte[65536];
        long total = 0;
        for (int n; (n = in.read(buffer)) != -1;) {
            total += n;
            if (total > MAX_BYTES) throw new IllegalArgumentException("This file is larger than 256 MB");
            out.write(buffer, 0, n);
        }
    }

    private void notice(String action, Exception error) {
        notices.offer(action + ": " + (error.getMessage() == null ? "The file operation failed" : error.getMessage()));
    }

    private void clearExport() {
        if (exporting != null) { exporting.delete(); exporting = null; }
    }

    @Override protected void onDestroy() {
        files.shutdown();
        clearExport();
        super.onDestroy();
    }
}

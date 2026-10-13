package io.github.storytold.cadcraft;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import android.webkit.MimeTypeMap;
import java.io.File;
import java.io.FileNotFoundException;
import java.util.List;
import java.util.Locale;

/** Read-only, explicitly granted snapshots. No access to private drawings or recovery. */
public final class ShareProvider extends ContentProvider {
    @Override public boolean onCreate() { return true; }
    private File resolve(Uri uri) throws FileNotFoundException {
        try {
            if (getContext() == null || !(getContext().getPackageName() + ".share").equals(uri.getAuthority())) throw new FileNotFoundException();
            List<String> parts = uri.getPathSegments();
            if (parts.size() != 2 || parts.get(0).contains("/") || parts.get(1).contains("/")) throw new FileNotFoundException();
            File root = new File(getContext().getCacheDir(), "shared").getCanonicalFile();
            File path = new File(new File(root, parts.get(0)), parts.get(1)).getCanonicalFile();
            if (!path.getPath().startsWith(root.getPath() + File.separator) || !path.isFile()) throw new FileNotFoundException();
            return path;
        } catch (Exception error) { throw new FileNotFoundException("Shared snapshot unavailable"); }
    }
    @Override public String getType(Uri uri) {
        String name = uri.getLastPathSegment(); if (name == null) return "application/octet-stream";
        String mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(name.substring(name.lastIndexOf('.') + 1).toLowerCase(Locale.ROOT));
        return mime == null ? "application/octet-stream" : mime;
    }
    @Override public Cursor query(Uri uri, String[] projection, String selection, String[] args, String sort) {
        try {
            File path = resolve(uri);
            String[] columns = projection == null ? new String[]{OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE} : projection;
            MatrixCursor cursor = new MatrixCursor(columns); Object[] values = new Object[columns.length];
            for (int i = 0; i < columns.length; i++) {
                if (OpenableColumns.DISPLAY_NAME.equals(columns[i])) values[i] = path.getName();
                else if (OpenableColumns.SIZE.equals(columns[i])) values[i] = path.length();
            }
            cursor.addRow(values); return cursor;
        } catch (FileNotFoundException error) { return null; }
    }
    @Override public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        if (!"r".equals(mode)) throw new FileNotFoundException("Read-only snapshot");
        return ParcelFileDescriptor.open(resolve(uri), ParcelFileDescriptor.MODE_READ_ONLY);
    }
    @Override public Uri insert(Uri uri, ContentValues values) { throw new UnsupportedOperationException("Read-only snapshot"); }
    @Override public int delete(Uri uri, String selection, String[] args) { return 0; }
    @Override public int update(Uri uri, ContentValues values, String selection, String[] args) { return 0; }
}

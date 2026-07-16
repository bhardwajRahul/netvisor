// Immediate custom action: read the per-tenant values the server encoded into the
// MSI filename and set them as installer properties, so a provisioned download is a
// double-click install with only the API key left to enter.
//
// Renaming a signed MSI does not break its Authenticode signature (the signature covers
// the file bytes, not the name), so the server ships ONE signed MSI and renames a copy
// per daemon. The filename looks like:
//
//   scanopy-daemon~~mode=<hex>~~name=<hex>~~url=<hex>~~addr=<hex>~~port=<hex>~~loglevel=<hex>~~logfile=<hex>.msi
//
// Only the segments the server knows are present. Values are hex-encoded (0-9a-f) so any
// value (URLs with ://, Windows paths with :\) survives Windows filename rules, and JScript
// can decode them without a base64 dependency. The API KEY is deliberately NOT encoded here
// — a live credential must never sit in a filename / Downloads folder / MSI log.

function DecodeHex(hex) {
    var out = '';
    for (var i = 0; i + 1 < hex.length; i += 2) {
        out += String.fromCharCode(parseInt(hex.substr(i, 2), 16));
    }
    return out;
}

function ParseFilename() {
    // Full path of the MSI currently executing (the renamed download at first install).
    var dbPath = Session.Property('OriginalDatabase');
    if (!dbPath) {
        return 1; // ERROR_SUCCESS-equivalent for JScript CA: nothing to do
    }

    // Strip directory and the .msi extension.
    var base = dbPath.replace(/^.*[\\\/]/, '').replace(/\.msi$/i, '');

    // Map filename segment keys to installer properties.
    var map = {
        mode: 'MODE',
        name: 'DAEMONNAME',
        url: 'SERVERURL',
        addr: 'LISTENADDRESS',
        port: 'LISTENPORT',
        loglevel: 'LOGLEVEL',
        logfile: 'LOGFILE'
    };

    var segments = base.split('~~');
    for (var i = 0; i < segments.length; i++) {
        var seg = segments[i];
        var eq = seg.indexOf('=');
        if (eq < 0) {
            continue;
        }
        var key = seg.substr(0, eq);
        var prop = map[key];
        if (!prop) {
            continue;
        }
        var value = DecodeHex(seg.substr(eq + 1));
        if (value) {
            Session.Property(prop) = value;
        }
    }

    return 1;
}

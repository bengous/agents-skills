# Printing the plan

Printing sends the plan to a device: run it only when the user asked for a print.

## Through CUPS

`lpstat -e` lists every destination, including network printers CUPS can reach without a configured queue. Then:

```bash
lp -d <destination> -o media=A4 -o sides=one-sided -o fit-to-page plan.pdf
```

## When `lp` fails on a driverless printer

`lp: No such file or directory`, with `Bad device URI "://"` and `CUPS-Create-Local-Printer` in `/var/log/cups/error_log`, means CUPS failed to create its temporary queue for a printer found over DNS-SD. The lasting fix is a permanent queue, a system setting the user adds themselves:

```bash
sudo lpadmin -p <name> -E -v ipp://<address>/<rp> -m everywhere
```

Until then, send the pages to the printer over IPP with `ipptool`, which ships with CUPS:

1. Find the printer: `avahi-browse -rt _ipp._tcp` gives its `address`, `port` and the `rp=` path in `txt`.
2. Read what it accepts: `ipptool -tv ipp://<address>/<rp> get-printer-attributes.test | grep -E 'printer-state |document-format-supported|media-ready'`. Done when it is `idle` or `processing` and lists `image/jpeg` (or `application/pdf`: then send the PDF itself in step 4).
3. Rasterise each sheet and turn it to portrait: `pdftoppm -r 300 -gray -jpeg -jpegopt quality=95 plan.pdf page`, then `magick page-1.jpg -rotate 90 p-1.jpg` for each page.
4. Send each page: `ipptool -tv -f p-1.jpg ipp://<address>/<rp> <skill-dir>/scripts/print-job.test`. Done when each run shows `[PASS]` and a `job-id`. The test asks for A4: change `media` for another paper size.
5. Confirm: `get-printer-attributes.test` shows `printer-state` `processing` and the `queued-job-count`, then `idle` once the sheets are out. Tell the user to check the tray: nothing here sees the paper.

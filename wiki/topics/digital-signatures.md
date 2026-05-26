---
title: "Digital Signatures in PDF"
kind: "knowledge"
category: "wiki"
summary: "How digital signatures work in PDF: PAdES profiles, PKCS#7/CMS structure, Rust signing libraries, and what is feasible for pdfp Stage 4."
entities: [PAdES, PKCS7, underskrift, pdfluent_sign, digital-signature, CMS]
---

# Digital Signatures in PDF

PDF digital signatures are a standardized mechanism for cryptographically signing documents. They support visible and invisible signatures, multiple signers, certification (document-level), long-term validation, and timestamping. For `pdfp`, they are a Stage 4 capability — not urgent, but well-supported by existing Rust crates when the time comes.

---

## How PDF Signatures Work

A PDF signature is stored in a **signature field** (AcroForm field with `/FT /Sig`) backed by a **signature dictionary** in the document's signature handler.

The signature dictionary contains:

| Entry | Description |
|---|---|
| `/Filter` | Handler name — `Adobe.PPKLite` or `Adobe.PPKMS` |
| `/SubFilter` | Signature format — `adbe.pkcs7.detached` (PKCS#7) or `ETSI.CAdES.detached` (PAdES) |
| `/Contents` | The CMS/PKCS#7 signature blob (detached signature over the document's byte range) |
| `/ByteRange` | Array of 4 integers defining the signed byte range (everything EXCEPT the `/Contents` value) |
| `/Cert` | Optional: signer's certificate (if not embedded in CMS) |
| `/M` | Signing time |
| `/Name` | Signer name |
| `/Reason` | Signing reason |
| `/Location` | Signing location |
| `/V` | PAdES version level (1, 2, 3, 4 for LTV) |

The signed byte range covers the entire file except the `/Contents` value — this allows the signature to be embedded without invalidating itself. Signing is an **incremental save**: the original file is unchanged, and the signature appears in an appended cross-reference section.

---

## PAdES Profiles

**PAdES** (PDF Advanced Electronic Signatures, ETSI EN 319 142) defines four baseline conformance levels:

| Level | Name | Description |
|---|---|---|
| B-B | Basic | CMS/PKCS#7 detached signature. Minimum for PAdES. |
| B-T | Timestamp | B-B + trusted timestamp (RFC 3161) proving signing time. |
| B-LT | Long-Term | B-T + certificate chain and revocation data embedded in document. |
| B-LTA | Long-Term with Archive | B-LT + periodic re-timestamping with archive timestamps. Survives certificate expiry. |

For `pdfp`, B-B and B-T are the realistic targets. B-LT/LTA require CRL/OCSP embedding and periodic re-timestamping — operational complexity beyond the scope of a conversion tool.

---

## Rust Libraries

### underskrift (crates.io/underskrift)

The primary Rust library for PDF signing. BSD-2-Clause licensed, production-grade, 0.1.3 as of March 2026.

**Capabilities**:
- PAdES B-B through B-LTA conformance
- PKCS#7 (traditional `adbe.pkcs7.detached`) and ETSI CAdES
- Visible and invisible signatures
- Multiple signatures and certification signatures
- Long-term validation (LTV) with document security stores (DSS) and document timestamps
- RFC 3161 timestamps, RFC 9321 SVT tokens, ETSI TS 119 102-2 validation reports
- Signature verification including certificate chain validation

**Usage**:
```rust
let pdf_bytes = std::fs::read("document.pdf")?;
let signer = SoftwareSigner::from_pkcs12_file("identity.p12", "password")?;
let signed = PdfSigner::new(&pdf_bytes)?
    .sign(&signer, SigningOptions::default())?;
std::fs::write("signed.pdf", signed)?;
```

### pdfluent_sign

Alternative library. Focus on validation and CMS parsing. Supports certificate chain verification and DocMDP/FieldMDP permission handling. Less mature API than underskrift for signing.

### lopdf

Can manipulate signature dictionaries at the PDF object level — creating fields, setting `/ByteRange`, inserting `/Contents`. Does NOT perform cryptographic operations. Useful as a fallback if you need full control over the PDF structure around the signature.

---

## Relevance to pdfp

### Signature detection (quick win)
Detect signature fields during extraction — `pdfp inspect` could list existing signatures with signer, date, validity status. Does not require cryptographic operations.

### Signing (Stage 4)
`pdfp sign --key identity.p12 --reason "Approved" document.pdf` — apply a digital signature to a PDF. `underskrift` handles all the cryptographic complexity.

### Verification (Stage 4)
`pdfp verify document.pdf` — check existing signatures for validity, certificate chain trust, and timestamp integrity.

### Certification
Certification signatures (DocMDP) lock the document against certain modifications after signing — relevant for forms and archival.

---

## Feasibility Assessment

| Aspect | Verdict |
|---|---|
| Rust library maturity | Good — `underskrift` is production-grade |
| API complexity | Low — signing is `read → sign → write` |
| Dependencies | OpenSSL or rustls for crypto; no system PDF tools needed |
| PAdES B-B support | Full via `underskrift` |
| Certificate management | Out of scope for `pdfp` — user provides PKCS#12 |
| Complexity risk | Low — well-understood standard, mature Rust ecosystem |

---

## Current State in pdfp

| Capability | Status |
|---|---|
| Signature field detection | Not implemented |
| Signature verification | Not implemented (Stage 4) |
| Document signing | Not implemented (Stage 4) |
| Certification support | Not evaluated |

## Related Pages

- [PDF Forms](forms.md) — signature fields are AcroForm sub-type
- [PDF Metadata](pdf-metadata.md) — XMP metadata may carry signature information

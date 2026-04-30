# Sida loo Releas-gareeyo MyGit (Distribution Guide)

Hambalyo! Hadda oo MyGit uu dhammaystiran yahay, halkan waa talaabooyinka aad u qaadayso si aad dadka kale ugu gudbiso.

## 1. Dhalinta Hal Fayl (Single Executable)
Halkii aad dadka u diri lahayd code-ka oo dhan, waxaad u diraysaa hal fayl oo ay ordon karaan.

### A. Isticmaalka Bun (Aad u dhakhso badan)
Haddii aad laptop-kaaga ku haysato Bun:
```bash
# Windows
bun build app/main.ts --compile --outfile mygit-windows.exe

# Linux
bun build app/main.ts --compile --outfile mygit-linux

# macOS
bun build app/main.ts --compile --outfile mygit-macos
```

### B. Isticmaalka `pkg` (Habka Node.js)
Haddii aad rabto inaad isticmaasho Node.js standard:
1. Install: `npm install -g pkg`
2. Build: `pkg .`
*Kani wuxuu kuu soo saarayaa 3 fayl oo kala ah Windows, Linux, iyo Mac.*

## 2. Ku Publish-garaynta NPM
Tani waa habka ugu habboon ee dadka developers-ka ah ay ku soo degsan karaan.
1. Gal [npmjs.com](https://www.npmjs.com) oo akoon ka samayso.
2. Terminal-ka ku qor: `npm login`
3. Markaad gasho, qor: `npm publish`
4. Dadka waxay ku soo degsan karaan: `npm install -g my-own-git`

## 3. Sameynta Installer (.msi ama .exe setup)
Haddii aad rabto "Setup Wizard":
*   **Windows**: Isticmaal [Inno Setup](https://jrsoftware.org/isinfo.php). Waa bilaash. Waxaad siinaysaa `mygit-windows.exe`-ga aad dhashay, isna wuxuu kuu samaynayaa `setup.exe`.
*   **Mac**: Isticmaal [Homebrew](https://brew.sh). Waxaad samaynaysaa wax loo yaqaan "Formula" si dadku u dhahaan `brew install mygit`.

## 4. GitHub Releases
Habka ugu caansan:
1. Code-kaaga geli GitHub.
2. Tag qaybta **Releases**.
3. Upload-garee faylasha aad dhashay (`mygit-windows.exe`, `mygit-linux`, iwm).
4. Dadka waxay si toos ah uga soo degsanayaan GitHub.

---
**Waa diyaar sxb! Hadda MyGit diyaar ayuu u yahay inuu caalamka ku faafo.** 🚀

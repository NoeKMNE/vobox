use std::io::Cursor;

use calamine::{open_workbook_auto_from_rs, DataType, Reader};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct ExploitationDraft {
    pub id_interne: Option<String>,
    pub id_national: Option<String>,
    pub village: Option<String>,
    pub district: Option<String>,
    pub region_inspection: Option<String>,
    pub superficie_total: Option<f64>,
    pub type_exploitation: Option<String>,
    pub nb_unites: Option<i64>,
    pub nb_cultures: Option<i64>,
    pub operateur_prenom: Option<String>,
    pub operateur_nom: Option<String>,
    pub operateur_telephone: Option<String>,
    pub operateur_id_national: Option<String>,
    pub operateur_genre: Option<String>,
    pub operateur_annee_nais: Option<i64>,
    pub proprietaire_prenom: Option<String>,
    pub proprietaire_nom: Option<String>,
    pub proprietaire_telephone: Option<String>,
    pub proprietaire_id_national: Option<String>,
    pub proprietaire_genre: Option<String>,
    pub travailleurs_permanents: Option<i64>,
    pub travailleurs_temporaires: Option<i64>,
    pub inspecteur_nom: Option<String>,
    pub inspecteur_annee: Option<i64>,
    pub inspecteur_mois: Option<i64>,
    pub inspecteur_jour: Option<i64>,
}

impl ExploitationDraft {
    pub fn empty() -> Self {
        Self {
            id_interne: None,
            id_national: None,
            village: None,
            district: None,
            region_inspection: None,
            superficie_total: None,
            type_exploitation: None,
            nb_unites: None,
            nb_cultures: None,
            operateur_prenom: None,
            operateur_nom: None,
            operateur_telephone: None,
            operateur_id_national: None,
            operateur_genre: None,
            operateur_annee_nais: None,
            proprietaire_prenom: None,
            proprietaire_nom: None,
            proprietaire_telephone: None,
            proprietaire_id_national: None,
            proprietaire_genre: None,
            travailleurs_permanents: None,
            travailleurs_temporaires: None,
            inspecteur_nom: None,
            inspecteur_annee: None,
            inspecteur_mois: None,
            inspecteur_jour: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CultureDraft {
    pub id_interne: Option<String>,
    pub produit: Option<String>,
    pub variete: Option<String>,
    pub superficie_ha: Option<f64>,
    pub recolte_estimee: Option<f64>,
    pub recolte_precedente: Option<f64>,
    pub volume_vendu: Option<f64>,
}

impl CultureDraft {
    pub fn empty() -> Self {
        Self {
            id_interne: None,
            produit: None,
            variete: None,
            superficie_ha: None,
            recolte_estimee: None,
            recolte_precedente: None,
            volume_vendu: None,
        }
    }
    fn is_empty(&self) -> bool {
        self.id_interne.is_none()
            && self.produit.is_none()
            && self.variete.is_none()
            && self.superficie_ha.is_none()
            && self.recolte_estimee.is_none()
            && self.recolte_precedente.is_none()
            && self.volume_vendu.is_none()
    }
}

#[derive(Clone, Debug)]
pub struct UniteDraft {
    pub id_interne: Option<String>,
    pub identifiant_unite: Option<String>,
    pub superficie_ha: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

impl UniteDraft {
    pub fn empty() -> Self {
        Self {
            id_interne: None,
            identifiant_unite: None,
            superficie_ha: None,
            latitude: None,
            longitude: None,
        }
    }
    fn is_empty(&self) -> bool {
        self.id_interne.is_none()
            && self.identifiant_unite.is_none()
            && self.superficie_ha.is_none()
            && self.latitude.is_none()
            && self.longitude.is_none()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ParsedWorkbook {
    pub exploitations: Vec<ExploitationDraft>,
    pub cultures: Vec<CultureDraft>,
    pub unites: Vec<UniteDraft>,
}

#[derive(Clone, Copy, Debug)]
enum ColKind {
    Field(&'static str),
    Repeated(GenLabel),
    Skip,
}

#[derive(Clone, Copy, Debug)]
enum GenLabel {
    Prenom,
    Nom,
    Telephone,
    IdNational,
    Genre,
}

pub fn parse_excel(bytes: &[u8]) -> Result<ParsedWorkbook, String> {
    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|e| format!("Format non pris en charge (fichier xlsx/xls attendu) : {e}"))?;

    let sheet_names = workbook.sheet_names().to_owned();
    let mut out = ParsedWorkbook::default();

    for sheet in sheet_names {
        let range = workbook
            .worksheet_range(&sheet)
            .ok_or_else(|| format!("Feuille « {sheet} » introuvable"))?
            .map_err(|e| format!("Feuille « {sheet} » illisible : {e}"))?;
        let rows: Vec<_> = range.rows().collect();

        if let Some((header_idx, kind)) = select_header(&rows) {
            match kind {
                2 => parse_cultures(&rows, header_idx, &mut out),
                3 => parse_unites(&rows, header_idx, &mut out),
                _ => parse_exploitations(&rows, header_idx, &mut out),
            }
        }
    }

    if out.exploitations.is_empty() && out.cultures.is_empty() && out.unites.is_empty() {
        let mut err = String::from(
            "Aucune donnée exploitable : vérifiez que le classeur contient les feuilles d'exploitation, de cultures ou d'unités agricoles",
        );
        if let Ok(dumps) = dump_excel(bytes) {
            for d in dumps {
                err.push_str(&format!("\n\n— Feuille « {} » ({} lignes)", d.name, d.rows));
                for row in &d.sample {
                    err.push_str(&format!("\n  {}", row.join(" | ")));
                }
            }
        }
        return Err(err);
    }
    Ok(out)
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct SheetDump {
    pub name: String,
    pub rows: usize,
    pub sample: Vec<Vec<String>>,
}

pub fn maj_estimations_xlsx(
    bytes: &[u8],
    ajustements: &[(String, f64)],
) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::Workbook;

    let cursor = Cursor::new(bytes);
    let mut src = open_workbook_auto_from_rs(cursor)
        .map_err(|e| format!("Format non pris en charge : {e}"))?;
    let names = src.sheet_names().to_owned();

    let mut out = Workbook::new();
    for name in names {
        let range = src
            .worksheet_range(&name)
            .ok_or_else(|| "Feuille introuvable".to_string())?
            .map_err(|e| format!("Lecture impossible : {e}"))?;

        // repérer la colonne estimation + produit via les en-têtes.
        let rows: Vec<&[DataType]> = range.rows().collect();
        let mut estimation_col: Option<usize> = None;
        let mut produit_col: Option<usize> = None;
        let mut header_idx = 0usize;
        for (i, row) in rows.iter().enumerate().take(20) {
            let headers: Vec<String> = row.iter().map(cell_str).collect();
            let est = headers.iter().position(|h| {
                let t = clean(h);
                t.contains("recolte") && (t.contains("cours") || t.contains("rapprochee"))
            });
            let pro = headers.iter().position(|h| clean(h).contains("produit"));
            if est.is_some() && pro.is_some() {
                estimation_col = est;
                produit_col = pro;
                header_idx = i;
                break;
            }
        }

        // appliquer les facteurs d'estimation aux lignes correspondantes.
        let mut adj_values: Vec<Vec<DataType>> = rows.iter().map(|r| r.to_vec()).collect();
        if let (Some(est_col), Some(prod_col)) = (estimation_col, produit_col) {
            for row in adj_values.iter_mut().skip(header_idx + 1) {
                let produit = cell_str(&row[prod_col]);
                let key = produit.trim().to_lowercase();
                if let Some((_, factor)) = ajustements.iter().find(|(p, _)| p.trim().to_lowercase() == key) {
                    if let Some(v) = cell_str(&row[est_col]).parse::<f64>().ok() {
                        row[est_col] = DataType::Float(v * factor);
                    }
                }
            }
        }

        let sheet = out.add_worksheet();
        sheet
            .set_name(&name)
            .map_err(|e| format!("Nom de feuille invalide : {e}"))?;
        for (r, row) in adj_values.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                match cell {
                    DataType::Float(v) => { sheet.write_number(r as u32, c as u16, *v).map_err(|e| e.to_string())?; }
                    DataType::Int(v) => { sheet.write_number(r as u32, c as u16, *v as f64).map_err(|e| e.to_string())?; }
                    DataType::Bool(b) => { sheet.write_boolean(r as u32, c as u16, *b).map_err(|e| e.to_string())?; }
                    DataType::String(s) => { sheet.write_string(r as u32, c as u16, s).map_err(|e| e.to_string())?; }
                    DataType::DateTime(v) => { let v = *v; let _ = sheet.write_number(r as u32, c as u16, v).map_err(|e| e.to_string()); }
                    _ => {}
                }
            }
        }
    }

    let buf = out
        .save_to_buffer()
        .map_err(|e| format!("Écriture impossible : {e}"))?;
    Ok(buf)
}

pub fn inspect_excel(bytes: &[u8]) -> Result<Vec<SheetDump>, String> {
    dump_excel(bytes)
}

fn dump_excel(bytes: &[u8]) -> Result<Vec<SheetDump>, String> {
    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|e| format!("Format non pris en charge : {e}"))?;
    let names = workbook.sheet_names().to_owned();
    let mut out = Vec::new();
    for name in names {
        let Ok(range) = workbook.worksheet_range(&name).ok_or(()) else {
            continue;
        };
        let Ok(range) = range else {
            continue;
        };
        let rows: Vec<&[DataType]> = range.rows().collect();
        let sample: Vec<Vec<String>> = rows
            .iter()
            .filter(|r| r.iter().any(|c| !matches!(c, DataType::Empty)))
            .take(5)
            .map(|r| r.iter().map(|c| truncate(&cell_str(c))).collect())
            .collect();
        out.push(SheetDump {
            name,
            rows: rows.len(),
            sample,
        });
    }
    Ok(out)
}

fn truncate(s: &str) -> String {
    let c: String = s.chars().take(70).collect();
    if c.chars().count() < s.chars().count() {
        format!("{c}…")
    } else {
        c
    }
}

fn select_header(rows: &[&[DataType]]) -> Option<(usize, i32)> {
    let mut best: Option<(usize, usize, i32)> = None;

    let candidates = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.iter().any(|c| !matches!(c, DataType::Empty)))
        .map(|(i, _)| i)
        .take(20);

    for idx in candidates {
        let headers: Vec<String> = rows[idx].iter().map(cell_str).collect();
        let scores = [
            headers
                .iter()
                .filter(|h| {
                    matches!(
                        sheet1_col_kind(h),
                        ColKind::Field(_) | ColKind::Repeated(_)
                    )
                })
                .count(),
            headers
                .iter()
                .filter(|h| matches!(sheet2_col_kind(h), ColKind::Field(_)))
                .count(),
            headers
                .iter()
                .filter(|h| matches!(sheet3_col_kind(h), ColKind::Field(_)))
                .count(),
        ];
        for (kind, score) in scores.iter().enumerate() {
            if *score < 3 {
                continue;
            }
            let replace = match best {
                None => true,
                Some((best_score, _, _)) => *score > best_score,
            };
            if replace {
                best = Some((*score, idx, kind as i32 + 1));
            }
        }
    }

    best.map(|(_, idx, kind)| (idx, kind))
}

fn parse_exploitations(
    rows: &[&[DataType]],
    header_idx: usize,
    out: &mut ParsedWorkbook,
) {
    let headers: Vec<String> = rows[header_idx].iter().map(cell_str).collect();
    let cols: Vec<ColKind> = headers.iter().map(|h| sheet1_col_kind(h)).collect();

    for row in rows.iter().skip(header_idx + 1) {
        if row.iter().all(|c| matches!(c, DataType::Empty)) {
            continue;
        }
        let mut d = ExploitationDraft::empty();
        let mut counts: [usize; 5] = [0; 5];

        for (i, cell) in row.iter().enumerate() {
            let val = cell_str(cell);
            if val.trim().is_empty() {
                continue;
            }
            let field = match cols.get(i).copied().unwrap_or(ColKind::Skip) {
                ColKind::Field(f) => f,
                ColKind::Repeated(label) => repeated_field(label, &mut counts),
                ColKind::Skip => continue,
            };
            set_exploitation_field(&mut d, field, &val);
        }

        if d.id_interne.is_none() && d.operateur_nom.is_none() && d.village.is_none() {
            continue;
        }
        out.exploitations.push(d);
    }
}

fn repeated_field(label: GenLabel, counts: &mut [usize; 5]) -> &'static str {
    let (idx, first, second): (usize, &'static str, &'static str) = match label {
        GenLabel::Prenom => (0, "operateur_prenom", "proprietaire_prenom"),
        GenLabel::Nom => (1, "operateur_nom", "proprietaire_nom"),
        GenLabel::Telephone => (2, "operateur_telephone", "proprietaire_telephone"),
        GenLabel::IdNational => (3, "operateur_id_national", "proprietaire_id_national"),
        GenLabel::Genre => (4, "operateur_genre", "proprietaire_genre"),
    };
    let field = if counts[idx] == 0 { first } else { second };
    counts[idx] += 1;
    field
}

fn sheet1_col_kind(h: &str) -> ColKind {
    let h = clean(h);
    if h.is_empty() {
        return ColKind::Skip;
    }
    use ColKind::*;
    if h.contains("identifiant interne") || h.contains("id interne") {
        Field("id_interne")
    } else if h.contains("identifiant national") || h.contains("id national") {
        Field("id_national")
    } else if h.contains("village") || h.contains("ville") || h.contains("adresse") || h.contains("localite") {
        Field("village")
    } else if h.contains("district") || h.contains("province") {
        Field("district")
    } else if h.contains("region") {
        Field("region_inspection")
    } else if h.contains("superficie") {
        Field("superficie_total")
    } else if h.contains("type") && h.contains("exploitation") {
        Field("type_exploitation")
    } else if h.contains("nombre") && h.contains("unites") {
        Field("nb_unites")
    } else if h.contains("nombre de cultures") {
        Field("nb_cultures")
    } else if h.contains("annee de naissance") || (h.contains("annee") && h.contains("nais")) {
        Field("operateur_annee_nais")
    } else if h.contains("nom de l'inspecteur") {
        Field("inspecteur_nom")
    } else if h.contains("inspection interne") && h.contains("annee") {
        Field("inspecteur_annee")
    } else if h.contains("inspection interne") && h.contains("mois") {
        Field("inspecteur_mois")
    } else if h.contains("inspection interne") && h.contains("jour") {
        Field("inspecteur_jour")
    } else if h.contains("travailleurs permanents") {
        Field("travailleurs_permanents")
    } else if h.contains("travailleurs temporaires") {
        Field("travailleurs_temporaires")
    } else if h.contains("telephone") || h == "num" {
        Repeated(GenLabel::Telephone)
    } else if h.contains("numero d'identification") {
        Repeated(GenLabel::IdNational)
    } else if h.contains("prenom") {
        Repeated(GenLabel::Prenom)
    } else if h.contains("genre") {
        Repeated(GenLabel::Genre)
    } else if has_word(&h, "nom") {
        Repeated(GenLabel::Nom)
    } else {
        Skip
    }
}

fn has_word(h: &str, word: &str) -> bool {
    h.split(|c: char| !c.is_alphanumeric()).any(|t| t == word)
}

fn set_exploitation_field(d: &mut ExploitationDraft, field: &'static str, val: &str) {
    let set_str = |slot: &mut Option<String>| {
        if slot.is_none() {
            *slot = Some(val.to_string());
        }
    };
    match field {
        "id_interne" => set_str(&mut d.id_interne),
        "id_national" => set_str(&mut d.id_national),
        "village" => set_str(&mut d.village),
        "district" => set_str(&mut d.district),
        "region_inspection" => set_str(&mut d.region_inspection),
        "type_exploitation" => set_str(&mut d.type_exploitation),
        "operateur_prenom" => set_str(&mut d.operateur_prenom),
        "operateur_nom" => set_str(&mut d.operateur_nom),
        "operateur_telephone" => set_str(&mut d.operateur_telephone),
        "operateur_id_national" => set_str(&mut d.operateur_id_national),
        "operateur_genre" => set_str(&mut d.operateur_genre),
        "proprietaire_prenom" => set_str(&mut d.proprietaire_prenom),
        "proprietaire_nom" => set_str(&mut d.proprietaire_nom),
        "proprietaire_telephone" => set_str(&mut d.proprietaire_telephone),
        "proprietaire_id_national" => set_str(&mut d.proprietaire_id_national),
        "proprietaire_genre" => set_str(&mut d.proprietaire_genre),
        "inspecteur_nom" => set_str(&mut d.inspecteur_nom),
        "superficie_total" => {
            if let Some(v) = parse_float(val) {
                d.superficie_total = Some(v);
            }
        }
        "nb_unites" => {
            if let Some(v) = parse_int(val) {
                d.nb_unites = Some(v);
            }
        }
        "nb_cultures" => {
            if let Some(v) = parse_int(val) {
                d.nb_cultures = Some(v);
            }
        }
        "operateur_annee_nais" => {
            if let Some(v) = parse_int(val) {
                d.operateur_annee_nais = Some(v);
            }
        }
        "travailleurs_permanents" => {
            if let Some(v) = parse_int(val) {
                d.travailleurs_permanents = Some(v);
            }
        }
        "travailleurs_temporaires" => {
            if let Some(v) = parse_int(val) {
                d.travailleurs_temporaires = Some(v);
            }
        }
        "inspecteur_annee" => {
            if let Some(v) = parse_int(val) {
                d.inspecteur_annee = Some(v);
            }
        }
        "inspecteur_mois" => {
            if let Some(v) = parse_int(val) {
                d.inspecteur_mois = Some(v);
            }
        }
        "inspecteur_jour" => {
            if let Some(v) = parse_int(val) {
                d.inspecteur_jour = Some(v);
            }
        }
        _ => {}
    }
}

fn parse_cultures(rows: &[&[DataType]], header_idx: usize, out: &mut ParsedWorkbook) {
    let headers: Vec<String> = rows[header_idx].iter().map(cell_str).collect();
    let cols: Vec<ColKind> = headers.iter().map(|h| sheet2_col_kind(h)).collect();

    for row in rows.iter().skip(header_idx + 1) {
        if row.iter().all(|c| matches!(c, DataType::Empty)) {
            continue;
        }
        let mut d = CultureDraft::empty();
        for (i, cell) in row.iter().enumerate() {
            let val = cell_str(cell);
            if val.trim().is_empty() {
                continue;
            }
            let ColKind::Field(f) = cols.get(i).copied().unwrap_or(ColKind::Skip) else {
                continue;
            };
            set_culture_field(&mut d, f, &val);
        }
        if !d.is_empty() {
            out.cultures.push(d);
        }
    }
}

fn sheet2_col_kind(h: &str) -> ColKind {
    let h = clean(h);
    if h.is_empty() {
        return ColKind::Skip;
    }
    use ColKind::*;
    if h.contains("exploitation agricole") && h.contains("identifiant interne") {
        Field("id_interne")
    } else if h.contains("superficie") {
        Field("superficie_ha")
    } else if h.contains("variete") {
        Field("variete")
    } else if h.contains("produit") {
        Field("produit")
    } else if h.contains("recolte") && h.contains("cours") {
        Field("recolte_estimee")
    } else if h.contains("recolte") && h.contains("precedente") {
        Field("recolte_precedente")
    } else if h.contains("superieur") || h.contains("croissance") || h.contains("rendement") {
        Skip
    } else if h.contains("volume") {
        Field("volume_vendu")
    } else {
        Skip
    }
}

fn set_culture_field(d: &mut CultureDraft, field: &'static str, val: &str) {
    match field {
        "id_interne" => d.id_interne = Some(val.to_string()),
        "produit" => d.produit = Some(val.to_string()),
        "variete" => d.variete = Some(val.to_string()),
        "superficie_ha" => {
            if let Some(v) = parse_float(val) {
                d.superficie_ha = Some(v);
            }
        }
        "recolte_estimee" => {
            if let Some(v) = parse_float(val) {
                d.recolte_estimee = Some(v);
            }
        }
        "recolte_precedente" => {
            if let Some(v) = parse_float(val) {
                d.recolte_precedente = Some(v);
            }
        }
        "volume_vendu" => {
            if let Some(v) = parse_float(val) {
                d.volume_vendu = Some(v);
            }
        }
        _ => {}
    }
}

fn parse_unites(rows: &[&[DataType]], header_idx: usize, out: &mut ParsedWorkbook) {
    let headers: Vec<String> = rows[header_idx].iter().map(cell_str).collect();
    let cols: Vec<ColKind> = headers.iter().map(|h| sheet3_col_kind(h)).collect();

    for row in rows.iter().skip(header_idx + 1) {
        if row.iter().all(|c| matches!(c, DataType::Empty)) {
            continue;
        }
        let mut d = UniteDraft::empty();
        for (i, cell) in row.iter().enumerate() {
            let val = cell_str(cell);
            if val.trim().is_empty() {
                continue;
            }
            let ColKind::Field(f) = cols.get(i).copied().unwrap_or(ColKind::Skip) else {
                continue;
            };
            set_unite_field(&mut d, f, &val);
        }
        if !d.is_empty() {
            out.unites.push(d);
        }
    }
}

fn sheet3_col_kind(h: &str) -> ColKind {
    let h = clean(h);
    if h.is_empty() {
        return ColKind::Skip;
    }
    if h.contains("verification") || h.contains("s'agit") {
        return ColKind::Skip;
    }
    use ColKind::*;
    if h.contains("exploitation agricole") && h.contains("identifiant interne") {
        Field("id_interne")
    } else if h.contains("latitude") || h.contains("longitude") {
        if h.contains("verification") {
            Skip
        } else {
            Field(if h.contains("longitude") { "longitude" } else { "latitude" })
        }
    } else if h.contains("superficie") {
        Field("superficie_ha")
    } else if h.contains("identifiant") && h.contains("unite") || h.contains("unite agricole") {
        Field("identifiant_unite")
    } else {
        Skip
    }
}

fn set_unite_field(d: &mut UniteDraft, field: &'static str, val: &str) {
    match field {
        "id_interne" => d.id_interne = Some(val.to_string()),
        "identifiant_unite" => d.identifiant_unite = Some(val.to_string()),
        "superficie_ha" => {
            if let Some(v) = parse_float(val) {
                d.superficie_ha = Some(v);
            }
        }
        "latitude" => {
            if let Some(v) = parse_float(val) {
                d.latitude = Some(v);
            }
        }
        "longitude" => {
            if let Some(v) = parse_float(val) {
                d.longitude = Some(v);
            }
        }
        _ => {}
    }
}

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'À' | 'Â' => 'a',
            'à' | 'â' => 'a',
            'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'Î' | 'Ï' => 'i',
            'î' | 'ï' => 'i',
            'Ô' | 'Ö' => 'o',
            'ô' | 'ö' => 'o',
            'Ù' | 'Û' | 'Ü' => 'u',
            'ù' | 'û' | 'ü' => 'u',
            'Ç' => 'c',
            'ç' => 'c',
            'Œ' | 'œ' => 'o',
            '’' | '‘' | '`' => '\'',
            '\u{00a0}' => ' ',
            '_' => ' ',
            c => c,
        })
        .collect::<String>()
        .to_lowercase()
}

fn cell_str(cell: &DataType) -> String {
    match cell {
        DataType::Empty => String::new(),
        DataType::String(s) => s.trim().to_string(),
        DataType::Int(i) => i.to_string(),
        DataType::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        DataType::Bool(b) => b.to_string(),
        DataType::DateTime(serial) => excel_date(*serial),
        _ => String::new(),
    }
}

fn parse_float(s: &str) -> Option<f64> {
    let t = s.trim().replace([' ', '\u{00a0}'], "").replace(',', ".");
    if t.is_empty() {
        return None;
    }
    let mut end = 0;
    let mut seen_dot = false;
    for (i, &b) in t.as_bytes().iter().enumerate() {
        if b.is_ascii_digit() {
            end = i + 1;
        } else if b == b'-' && i == 0 {
            end = 1;
        } else if b == b'.' && !seen_dot {
            seen_dot = true;
            end = i + 1;
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    t[..end].parse().ok()
}

fn parse_int(s: &str) -> Option<i64> {
    parse_float(s).map(|f| f as i64)
}

fn excel_date(serial: f64) -> String {
    civil_from_days(serial.floor() as i64 - 25569)
}

fn civil_from_days(days: i64) -> String {
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::DataType;

    fn s(v: &str) -> DataType {
        DataType::String(v.into())
    }

    fn matrix(headers: &[&str], data: &[Vec<&str>]) -> Vec<Vec<DataType>> {
        let mut out = Vec::new();
        out.push(headers.iter().map(|h| s(h)).collect());
        for row in data {
            out.push(row.iter().map(|c| s(c)).collect());
        }
        out
    }

    #[test]
    fn feuille_plate_producteur() {
        let headers = [
            "id_interne",
            "id_national",
            "adresse",
            "district",
            "region",
            "superficie_parcelle",
            "type_exploitation",
            "nom",
            "prenom",
            "num",
            "numero_id_national",
            "genre",
            "annee_nais",
        ];
        let data = [
            vec![
                "B-207", "NF-88", "Kpengla", "Cotonou", "Littoral",
                "3,2", "grande", "Kone", "Adama", "0700000002", "NF-88-207",
                "M", "1978",
            ],
        ];
        let rows = matrix(&headers, &data);
        let refs: Vec<&[DataType]> = rows.iter().map(|r| r.as_slice()).collect();

        let mut out = ParsedWorkbook::default();
        parse_exploitations(&refs, 0, &mut out);

        assert_eq!(out.exploitations.len(), 1);
        let e = &out.exploitations[0];
        assert_eq!(e.id_interne.as_deref(), Some("B-207"));
        assert_eq!(e.id_national.as_deref(), Some("NF-88"));
        assert_eq!(e.village.as_deref(), Some("Kpengla"));
        assert_eq!(e.district.as_deref(), Some("Cotonou"));
        assert_eq!(e.region_inspection.as_deref(), Some("Littoral"));
        assert_eq!(e.superficie_total, Some(3.2));
        assert_eq!(e.type_exploitation.as_deref(), Some("grande"));
        assert_eq!(e.operateur_nom.as_deref(), Some("Kone"));
        assert_eq!(e.operateur_prenom.as_deref(), Some("Adama"));
        assert_eq!(e.operateur_telephone.as_deref(), Some("0700000002"));
        assert_eq!(e.operateur_genre.as_deref(), Some("M"));
        assert_eq!(e.operateur_annee_nais, Some(1978));
    }

    #[test]
    fn headers_en_ligne_2() {
        let title = vec![
            "EXPLOITATION AGRICOLE",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "OPÉRATEUR DE L'EXPLOITATION AGRICOLE",
            "",
            "",
            "",
            "",
            "",
            "PROPRIÉTAIRE DE L'EXPLOITATION AGRICOLE",
            "",
            "",
            "",
            "",
            "TRAVAILLEURS PAR L'EXPLOITATION AGRICOLE",
            "",
            "DONNÉES D'INSPECTION INTERNE",
            "",
            "",
            "",
        ];
        let headers = [
            "1. Identifiant interne unique d'exploitation agricole*",
            "2. Identifiant national d'exploitation agricole**",
            "3. Village*/Ville",
            "4. District/État/Province*",
            "5. Région d'inspection interne",
            "6. Superficie totale de l'exploitation agricole (ha)*",
            "7. Type d'exploitation agricole (petite/grande)*",
            "8. Nombre d'unités agricoles pour cette exploitation*",
            "9. Nombre de cultures certifiées",
            "10. Prénom*",
            "11. Nom*",
            "12. Numéro de téléphone**",
            "13. Numéro d'identification national**",
            "14. Genre*",
            "15. Année de naissance*",
            "16. Prénom",
            "17. Nom",
            "18. Numéro de téléphone",
            "19. Numéro d'identification national",
            "20. Genre",
            "21. Nombre de travailleurs permanents*",
            "22. Estimation du nombre de travailleurs temporaires par an*",
            "23. Nom de l'inspecteur interne*",
            "24. Année de l'inspection interne*",
            "25. Mois de l'inspection interne*",
            "26. Jour de l'inspection interne*",
        ];
        let data = vec![
            "CAPKO_001", "NON DISPONIBLE", "KOKOLOPOZO", "SASANDRA", "GBOKLE",
            "2,47", "Petite plantation", "1", "2022", "ALIOU", "TRAORE",
            "0103315797", "BF384003001001006413", "HOMME", "1971",
            "", "", "", "", "HOMME", "0", "0", "SOUGUE DAOUDA", "2025", "12", "23",
        ];

        let rows: Vec<Vec<DataType>> = vec![
            title.iter().map(|c| s(c)).collect(),
            headers.iter().map(|c| s(c)).collect(),
            data.iter().map(|c| s(c)).collect(),
        ];

        let refs: Vec<&[DataType]> = rows.iter().map(|r| r.as_slice()).collect();

        let (idx, kind) = select_header(&refs).expect("en-tête attendu");
        assert_eq!(idx, 1);
        assert_eq!(kind, 1);

        let mut out = ParsedWorkbook::default();
        parse_exploitations(&refs, idx, &mut out);
        assert_eq!(out.exploitations.len(), 1);
        let e = &out.exploitations[0];
        assert_eq!(e.id_interne.as_deref(), Some("CAPKO_001"));
        assert_eq!(e.operateur_nom.as_deref(), Some("TRAORE"));
        assert_eq!(e.operateur_telephone.as_deref(), Some("0103315797"));
        assert_eq!(e.proprietaire_genre.as_deref(), Some("HOMME"));
        assert_eq!(e.inspecteur_annee, Some(2025));
        assert_eq!(e.inspecteur_jour, Some(23));
    }

    #[test]
    fn feuille_prose_ignoree() {
        let rows_data = [
            vec!["Nom du document\u{a0}:", "", "", "", "", "Code du document\u{a0}:", "", "Version :"],
            vec!["Le registre des membres du groupe", "", "", "", "", "", "", ""],
            vec![
                "",
                "1) Veuillez compléter les informations des onglets 1. Membre du groupe, 2. Produit agricole certifié et 3. Unité agricole.",
                "",
                "",
            ],
            vec!["2) Des cellules formatées sont utilisées et de courtes explications peuvent s'afficher.", "", "", ""],
        ];
        let rows: Vec<Vec<DataType>> = rows_data.iter().map(|r| r.iter().map(|c| s(c)).collect()).collect();
        let refs: Vec<&[DataType]> = rows.iter().map(|r| r.as_slice()).collect();
        assert!(select_header(&refs).is_none());
    }

    #[test]
    fn detection_type_par_score() {
        let sheet1 = matrix(
            &["1. Identifiant interne unique d'exploitation agricole*",
                "3. Village*/Ville",
                "10. Prénom*",
            ],
            &[vec!["CAPKO_001", "KOKOLOPOZO", "ALIOU"]],
        );
        let refs: Vec<&[DataType]> = sheet1.iter().map(|r| r.as_slice()).collect();
        let (idx, kind) = select_header(&refs).expect("en-tête attendu");
        assert_eq!(idx, 0);
        assert_eq!(kind, 1);

        let sheet2 = matrix(
            &[
                "1. Identifiant interne unique d'exploitation agricole*",
                "2. Produit agricole certifié*",
                "5. Récolte de l'année en cours*",
                "7. Volume vendu*",
            ],
            &[vec!["CAPKO_001", "Cacao", "1343", "251"]],
        );
        let refs: Vec<&[DataType]> = sheet2.iter().map(|r| r.as_slice()).collect();
        let (idx, kind) = select_header(&refs).expect("en-tête attendu");
        assert_eq!(idx, 0);
        assert_eq!(kind, 2);

        let sheet3 = matrix(
            &[
                "1. Identifiant interne unique d'exploitation agricole*",
                "2. Identifiant d'unité agricole*",
                "4. Latitude",
                "5. Longitude",
            ],
            &[vec!["CAPKO_001", "CAPKO_001_1", "5.14", "-6.07"]],
        );
        let refs: Vec<&[DataType]> = sheet3.iter().map(|r| r.as_slice()).collect();
        let (idx, kind) = select_header(&refs).expect("en-tête attendu");
        assert_eq!(idx, 0);
        assert_eq!(kind, 3);
    }

    #[test]
    fn feuille1_exploitations() {
        let headers = [
            "1. Identifiant interne unique d'exploitation agricole*",
            "2. Identifiant national d'exploitation agricole**",
            "3. Village*/Ville",
            "4. District/État/Province*",
            "5. Région d'inspection interne",
            "6. Superficie totale de l'exploitation agricole (ha)*",
            "7. Type d'exploitation agricole (petite/grande)*",
            "8. Nombre d'unités agricoles pour cette exploitation*",
            "9. Nombre de cultures certifiées",
            "10. Prénom*",
            "11. Nom*",
            "12. Numéro de téléphone**",
            "13. Numéro d'identification national**",
            "14. Genre*",
            "15. Année de naissance*",
            "16. Prénom",
            "17. Nom",
            "18. Numéro de téléphone",
            "19. Numéro d'identification national",
            "20. Genre",
            "21. Nombre de travailleurs permanents*",
            "22. Estimation du nombre de travailleurs temporaires par an*",
            "23. Nom de l'inspecteur interne*",
            "24. Année de l'inspection interne*",
            "25. Mois de l'inspection interne*",
            "26. Jour de l'inspection interne*",
        ];
        let data = [
            vec![
                "A-001", "", "Koutia", "Korhogo", "Nord", "12,5", "petite",
                "1", "2", "Awa", "Kone", "0700000000", "CI-XYZ", "F", "1985",
                "Ibrahim", "Kone", "0700000001", "CI-ABC", "H",
                "3", "10", "M. Traore", "2024", "5", "12",
            ],
        ];
        let rows = matrix(&headers, &data);
        let refs: Vec<&[DataType]> = rows.iter().map(|r| r.as_slice()).collect();

        let mut out = ParsedWorkbook::default();
        parse_exploitations(&refs, 0, &mut out);

        assert_eq!(out.exploitations.len(), 1);
        let e = &out.exploitations[0];
        assert_eq!(e.id_interne.as_deref(), Some("A-001"));
        assert_eq!(e.village.as_deref(), Some("Koutia"));
        assert_eq!(e.district.as_deref(), Some("Korhogo"));
        assert_eq!(e.region_inspection.as_deref(), Some("Nord"));
        assert_eq!(e.superficie_total, Some(12.5));
        assert_eq!(e.type_exploitation.as_deref(), Some("petite"));
        assert_eq!(e.nb_unites, Some(1));
        assert_eq!(e.nb_cultures, Some(2));
        assert_eq!(e.operateur_prenom.as_deref(), Some("Awa"));
        assert_eq!(e.operateur_nom.as_deref(), Some("Kone"));
        assert_eq!(e.operateur_telephone.as_deref(), Some("0700000000"));
        assert_eq!(e.operateur_id_national.as_deref(), Some("CI-XYZ"));
        assert_eq!(e.operateur_genre.as_deref(), Some("F"));
        assert_eq!(e.operateur_annee_nais, Some(1985));
        assert_eq!(e.proprietaire_prenom.as_deref(), Some("Ibrahim"));
        assert_eq!(e.proprietaire_nom.as_deref(), Some("Kone"));
        assert_eq!(e.proprietaire_telephone.as_deref(), Some("0700000001"));
        assert_eq!(e.proprietaire_id_national.as_deref(), Some("CI-ABC"));
        assert_eq!(e.proprietaire_genre.as_deref(), Some("H"));
        assert_eq!(e.travailleurs_permanents, Some(3));
        assert_eq!(e.travailleurs_temporaires, Some(10));
        assert_eq!(e.inspecteur_nom.as_deref(), Some("M. Traore"));
        assert_eq!(e.inspecteur_annee, Some(2024));
        assert_eq!(e.inspecteur_mois, Some(5));
        assert_eq!(e.inspecteur_jour, Some(12));
    }

    #[test]
    fn feuille2_cultures() {
        let headers = [
            "1. Identifiant interne unique d'exploitation agricole*",
            "2. Produit agricole certifié*",
            "3. Variété**",
            "4. Superficie des produits agricoles certifiés * (ha)",
            "5. Estimation totale de la récolte de l'année en cours* (en kg)",
            "6. Récolte totale de l'année précédente* (en kg)",
            "7. Volume vendu/livré au groupe l'année précédente * (en kg)",
        ];
        let data = [
            vec!["A-001", "Cacao", "Amelioré", "6,2", "4500", "3900", "4100"],
            vec!["A-001", "Hévéa", "GT1", "4,0", "1500", "1400", "1200"],
        ];
        let rows = matrix(&headers, &data);
        let refs: Vec<&[DataType]> = rows.iter().map(|r| r.as_slice()).collect();

        let mut out = ParsedWorkbook::default();
        parse_cultures(&refs, 0, &mut out);

        assert_eq!(out.cultures.len(), 2);
        assert_eq!(out.cultures[0].id_interne.as_deref(), Some("A-001"));
        assert_eq!(out.cultures[0].produit.as_deref(), Some("Cacao"));
        assert_eq!(out.cultures[0].variete.as_deref(), Some("Amelioré"));
        assert_eq!(out.cultures[0].superficie_ha, Some(6.2));
        assert_eq!(out.cultures[0].recolte_estimee, Some(4500.0));
        assert_eq!(out.cultures[0].recolte_precedente, Some(3900.0));
        assert_eq!(out.cultures[0].volume_vendu, Some(4100.0));
    }

    #[test]
    fn feuille3_unites() {
        let headers = [
            "1. Identifiant interne unique d'exploitation agricole*",
            "2. Identifiant d'unité agricole*",
            "3. Superficie de l'unité agricole (ha)*",
            "4. Latitude (format DD)**",
            "5. Longitude (format DD)**",
        ];
        let data = [
            vec!["A-001", "UA-1", "8,4", "9,445664", "-5,646678"],
            vec!["A-001", "UA-2", "4,1", "", ""],
        ];
        let rows = matrix(&headers, &data);
        let refs: Vec<&[DataType]> = rows.iter().map(|r| r.as_slice()).collect();

        let mut out = ParsedWorkbook::default();
        parse_unites(&refs, 0, &mut out);

        assert_eq!(out.unites.len(), 2);
        assert_eq!(out.unites[0].identifiant_unite.as_deref(), Some("UA-1"));
        assert_eq!(out.unites[0].superficie_ha, Some(8.4));
        assert_eq!(out.unites[0].latitude, Some(9.445664));
        assert_eq!(out.unites[0].longitude, Some(-5.646678));
        assert_eq!(out.unites[1].superficie_ha, Some(4.1));
        assert_eq!(out.unites[1].latitude, None);
    }

    #[test]
    #[ignore]
    fn audit_fichier_reel_headers() {
        let path = r"C:\Users\DELL\Downloads\B-2025-GMR_2026_01_20_RA_00058972111 (1).xlsx";
        let bytes = std::fs::read(path).expect("fichier");
        let cursor = Cursor::new(bytes.clone());
        let mut workbook = open_workbook_auto_from_rs(cursor).expect("classeur");

        for sheet in workbook.sheet_names().to_owned() {
            let range = workbook.worksheet_range(&sheet).expect("feuille").expect("lisible");
            let rows: Vec<_> = range.rows().collect();
            println!("\n== FEUILLE « {sheet} » : {}", rows.len());

            let Some((hi, _)) = select_header(&rows) else {
                println!("  (prose, sans en-tête)");
                continue;
            };
            println!("  en-tête à la ligne {hi}");

            let headers: Vec<(usize, String)> = rows[hi]
                .iter()
                .enumerate()
                .map(|(i, c)| (i, clean(&cell_str(c))))
                .filter(|(_, v)| !v.is_empty())
                .collect();
            println!("  {} colonnes détectées :", headers.len());
            for (i, v) in headers {
                println!("    col[{i}] = « {v} »");
            }

            for k in 1..=2 {
                let Some(r) = rows.get(hi + k) else { break };
                let vals: Vec<(usize, String)> = r
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i, clean(&cell_str(c))))
                    .filter(|(_, v)| !v.is_empty())
                    .collect();
                if !vals.is_empty() {
                    println!("  ligne exemple +{k} :");
                    for (i, v) in vals {
                        println!("    col[{i}] = « {v} »");
                    }
                }
            }

            let norm = clean(&sheet);
            let classify = |h: &str| {
                if norm.contains("l'exploitation") {
                    sheet1_col_kind(h)
                } else if norm.contains("produit agricole") {
                    sheet2_col_kind(h)
                } else if norm.contains("unite agricole") {
                    sheet3_col_kind(h)
                } else {
                    ColKind::Skip
                }
            };
            if !(norm.contains("l'exploitation")
                || norm.contains("produit agricole")
                || norm.contains("unite agricole"))
            {
                continue;
            }
            println!("  colonnes IGNORÉES par le parseur :");
            let mut skipped_non_empty = 0usize;
            for (i, cell) in rows[hi].iter().enumerate() {
                let h = clean(&cell_str(cell));
                if h.is_empty() {
                    continue;
                }
                let ColKind::Skip = classify(&cell_str(cell)) else {
                    continue;
                };
                let filled = rows
                    .iter()
                    .skip(hi + 1)
                    .filter(|r| !matches!(cell_str(&(r.get(i).cloned().unwrap_or_default())).trim(), ""))
                    .count();
                println!("    col[{i}] « {h} » -> {filled} valeur(s) sur {}", rows.len() - hi - 1);
                skipped_non_empty += filled;
            }
            if skipped_non_empty == 0 {
                println!("    (aucune valeur dans ces colonnes)");
            }
        }

        let parsed = parse_excel(&bytes).expect("parse");
        println!(
            "\n== IMPORT : {} exploitations, {} cultures, {} unités agricoles",
            parsed.exploitations.len(),
            parsed.cultures.len(),
            parsed.unites.len()
        );
    }
}
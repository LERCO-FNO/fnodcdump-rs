use std::fs::File;
use std::{collections::HashSet, io, path::PathBuf};

use clap::{Parser, ValueEnum};
use dicom_core::VR::*;
use dicom_core::{DataDictionary, DataElement, Tag, dictionary::DataDictionaryEntry};
use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, StandardDataDictionary};

use serde::Serialize;
use walkdir::{self, WalkDir};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to input directory with DICOM files
    #[arg(short, long, value_parser = validate_input_path)]
    input_dir: PathBuf,

    /// Path to output file
    #[arg(short, long, default_value = "./tags.csv")]
    output_path: PathBuf,

    /// write tags at specified DICOM level
    #[arg(short, long, default_value = "study")]
    level: DicomLevel,

    /// tag preset
    #[arg(short, long)]
    preset: Vec<TagPreset>,

    /// output file type
    #[arg(short, long, default_value = "csv")]
    filetype: FileType,

    /// additional tags to write; compatible formats: gggg,eeee or TagName
    #[arg(short, long, value_parser = validate_tag)]
    tag: Vec<Tag>,
}

#[derive(Debug, Clone, ValueEnum)]
enum DicomLevel {
    Study,
    Series,
    Instance,
}

// improve the presets: what tags to include
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq, Hash)]
enum TagPreset {
    Patient,
    Study,
    Uid,
}

#[derive(Debug, Clone, ValueEnum)]
enum FileType {
    Csv,
    Json,
}

/*
- improve on the variants
- possibly add more variants for specific VRs
- possibly add variants for specific signed/unsigned integers
- add variants for decimal and float values
- for Error(String): see if possible to thing of better Error variant for missing values/tags, failed parsing, default values where needed/possible
*/
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ValueTypes {
    Integer(i64),
    UnInteger(u64),
    Float(f64),
    Text(String),
    Error(String), // see if possible to thing of better Error variant for missing values/tags, failed parsing, default values where needed/possible
}

#[derive(Serialize)]
struct TagData {
    header: Vec<String>,
    values: Vec<Vec<ValueTypes>>,
}

fn main() {
    let args = Args::parse();

    let files = get_dicom_files(args.input_dir);
    let tags = concat_tags(args.tag, args.preset);
    let mut uid_set: HashSet<String> = HashSet::new();
    let mut file_values: Vec<Vec<ValueTypes>> = Vec::new();

    // refactor whole loop into fn collect_tag_values(files: Vec<PathBuf>, tags: Vec<Tag>, level: DicomLevel) -> TagData
    for file in files {
        let ds = match dicom_object::open_file(file) {
            Ok(ds) => ds,
            Err(_) => continue,
        };

        let uid = match args.level {
            DicomLevel::Study => ds.element(tags::STUDY_INSTANCE_UID),
            DicomLevel::Series => ds.element(tags::SERIES_INSTANCE_UID),
            DicomLevel::Instance => ds.element(tags::SOP_INSTANCE_UID),
        }
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

        if uid_set.contains(&uid) {
            continue;
        }

        let mut values: Vec<ValueTypes> = Vec::new();

        for tag in &tags {
            let element = ds.element(*tag).unwrap();

            let value = match element.vr() {
                AE | CS | LO | LT | PN | SH | ST | UI => element
                    .to_str()
                    .map(|s| ValueTypes::Text(s.to_string()))
                    .unwrap_or(ValueTypes::Error(format!("invalid value"))),
                IS | SS => element
                    .to_int::<i64>()
                    .map(|s| ValueTypes::Integer(s))
                    .unwrap_or(ValueTypes::Error(format!("invalid integer value"))),

                AS => parse_age(element),

                _ => ValueTypes::Error(format!("unknown tag VR")),
            };

            values.push(value);
        }

        file_values.push(values);

        uid_set.insert(uid);
    }

    let tag_data = TagData {
        header: get_header(tags),
        values: file_values,
    };

    match args.filetype {
        // TODO: improve writing and error handling for both ValueType variants
        FileType::Csv => {
            let mut writer = csv::Writer::from_path("tags.csv").unwrap();
            writer.write_record(tag_data.header).unwrap();
            for vals in tag_data.values {
                writer.serialize(vals).unwrap();
            }
        }
        FileType::Json => {
            let file = File::create("tags.json").unwrap();
            serde_json::to_writer_pretty(file, &tag_data).unwrap();
        }
    }
}

fn parse_age(element: &DataElement<InMemDicomObject>) -> ValueTypes {
    let parsed_age = element.to_str().ok().and_then(|s| {
        let trimmed = s.trim();

        if trimmed.len() != 4 {
            return None;
        }

        // split at 4rd character (3nd index)
        let (num_part, _) = trimmed.split_at(3);

        num_part.parse::<i64>().ok()
    });

    match parsed_age {
        Some(age_int) => ValueTypes::Integer(age_int),
        None => ValueTypes::Error(format!(
            "unrecognized or malformed age string: {:?}",
            element.to_str().unwrap_or_default()
        )),
    }
}

fn tags_from_presets(preset_set: HashSet<TagPreset>) -> Vec<Tag> {
    let mut tags: Vec<Tag> = Vec::new();

    for preset in preset_set {
        match preset {
            TagPreset::Patient => tags.extend([tags::PATIENT_NAME, tags::PATIENT_AGE].into_iter()),
            TagPreset::Study => {
                tags.extend([tags::STUDY_INSTANCE_UID, tags::STUDY_DESCRIPTION].into_iter())
            }
            TagPreset::Uid => tags.extend([tags::STUDY_INSTANCE_UID, tags::SERIES_INSTANCE_UID]),
        }
    }

    tags
}

fn validate_input_path(path: &str) -> Result<PathBuf, io::Error> {
    let path = PathBuf::from(path);

    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Input path does not exist {}", path.display()),
        ));
    }

    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("Input path is not directory {}", path.display()),
        ));
    }

    Ok(path)
}

fn validate_tag(tag_string: &str) -> Result<Tag, String> {
    StandardDataDictionary
        .by_expr(tag_string)
        .map(|entry| entry.tag())
        .ok_or_else(|| "invalid DICOM tag keyword or (group,element)".to_string())
}

fn get_dicom_files(dir: PathBuf) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_str() != Some("DICOMDIR") && e.file_type().is_file())
        .map(|f| f.into_path())
        .collect()
}

fn concat_tags(tags_cli: Vec<Tag>, tag_presets: Vec<TagPreset>) -> Vec<Tag> {
    let mut tags: HashSet<Tag> = HashSet::from_iter(tags_cli);
    tags.extend(tags_from_presets(HashSet::from_iter(tag_presets)));
    tags.into_iter().collect()
}

fn get_header(tags: Vec<Tag>) -> Vec<String> {
    tags.iter()
        .map(|t| StandardDataDictionary.by_tag(*t).unwrap().alias.to_string())
        .collect()
}

/* TODO:
OVERALL:
- improve and reorder the functions
- add proper error handling
- improve if possible getting dicom elements and its error handling!!
- add test cases for future development!!
*/

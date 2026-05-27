# fnodcdump-rs

A tool for dumping DICOM tags for a bulk of studies. 

## Usage
```
fnodcdump --input-dir <INPUT_DIR> [options]
```

**Input/Output options:**
* `--input-dir (-i) <INPUT_DIR>`: Input directory.
* `--output-dir (-o) <OUTPUT_DIR>`: Output directory. Defaults to `<INPUT_DIR>/tags.csv` if not specified.
* `--filetype (-f) <FILETYPE>`: Output file type. Choose from `csv` (default) or `json`.

**Tag options:**
* `--level (-l) <LEVEL>`: Choose one DICOM level to write tags at:
  * `study`: Tags from the first file found in a study.
  * `series`: Tags from the first file found in every series, per study.
  * `instance`: Tags from every instance (file) in a study.

* `--preset (-p) <PRESET>`: Choose one or more tag presets **(will be extended)**:
  * `patient`: PatientName, PatientAge.
  * `study`: StudyInstanceUID, StudyDescription.
  * `uid`: StudyInstanceUID, SeriesInstanceUID.
* `--tag (-t) <TAG>`: Additional tag to write. Must be a valid tag and specified in format `gggg,eeee` or `TagName`.

Note: Order of columns of tags is not guaranteed to be the same as specified on command line or expected DICOM level, eg.: Study -> Series -> Instance!


## Example usage
`fnodcdump -i path/to/input/directory -o path/output/directory --level series --tag SeriesDescription --preset uid --filetype <FILETYPE>`

**Output for json:**
```
{
  "header": [
    "SeriesDescription",
    "StudyInstanceUID",
    "SeriesInstanceUID"
  ],
  "values": [
    ["Head", "Study UID 1", "Series UID 2"],
    ["Scout", "Study UID 1", "Series UID 2"],
    ["Abdomen", "Study UID 1", "Series UID 3"],
    ["Scout", "Study UID 2", "Series UID 1"],
    ["Full Body", "Study UID 2", "Series UID 2"],
  ]
}
```

**Output for csv:**
```
SeriesDescription,StudyInstanceUID,SeriesInstanceUID
Head,Study UID 1, Series UID 2,
Scout Study UID 1, Series UID 2,
Abdomen,Study UID 1, Series UID 2,
Scout Study UID 2, Series UID 1,
Full Body, Study UID 2, Series UID 2
```

If a tag is not found in DICOM file a `MissingTag` string is used as default value.
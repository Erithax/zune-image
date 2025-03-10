/*
 * Copyright (c) 2023.
 *
 * This software is free software;
 *
 * You can redistribute it or modify it under terms of the MIT, Apache License or Zlib license
 */

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read};
use std::path::Path;
use std::string::String;
use std::time::Instant;

use clap::parser::ValueSource::CommandLine;
use clap::ArgMatches;
use log::{debug, error, info, trace};
use zune_image::codecs::ImageFormat;
use zune_image::errors::ImageErrors;
use zune_image::pipelines::Pipeline;

use crate::cmd_args::CmdImageFormats;
use crate::cmd_parsers::global_options::CmdOptions;
use crate::cmd_parsers::{decoder_options, encoder_options};
use crate::file_io::{ZuneFile, ZuneMem};
use crate::probe_files::probe_input_files;
use crate::show_gui::open_in_default_app;

struct CmdPipeline {
    inner:   Pipeline,
    formats: Vec<ImageFormat>
}
impl CmdPipeline {
    pub fn new() -> CmdPipeline {
        CmdPipeline {
            inner:   Pipeline::new(),
            formats: vec![]
        }
    }
}

#[allow(unused_variables)]
#[allow(clippy::unused_io_amount)] // yes it's what I want
pub(crate) fn create_and_exec_workflow_from_cmd(
    args: &ArgMatches, cmd_opts: &CmdOptions
) -> Result<(), ImageErrors> {
    if let Some(view) = args.value_source("probe") {
        if view == CommandLine {
            probe_input_files(args);
            return Ok(());
        }
    }

    info!("Creating workflows from input");

    let decoder_options = decoder_options(args);
    let mut buf = [0; 30];

    for in_file in args.get_raw("in").unwrap() {
        let mut workflow: CmdPipeline = CmdPipeline::new();

        if in_file == "-" {
            // handle stdin
            let mut data = Vec::new();
            let bytes_read = std::io::stdin().read_to_end(&mut data)?;
            if let Some((format, _)) = ImageFormat::guess_format(std::io::Cursor::new(&data)) {
                if format.has_decoder() {
                    workflow
                        .inner
                        .chain_decoder(Box::new(ZuneMem::new(data, decoder_options)));
                } else {
                    return Err(ImageErrors::ImageDecoderNotImplemented(format));
                }
            } else {
                return Err(ImageErrors::ImageDecoderNotIncluded(ImageFormat::Unknown));
            }
        } else {
            File::open(in_file)?.read(&mut buf)?;

            add_operations(args, &mut workflow.inner)?;

            if let Some((format, _)) = ImageFormat::guess_format(std::io::Cursor::new(&buf)) {
                if format.has_decoder() {
                    workflow.inner.chain_decoder(Box::new(ZuneFile::new(
                        in_file.to_os_string(),
                        decoder_options
                    )));
                } else {
                    return Err(ImageErrors::ImageDecoderNotImplemented(format));
                }
            } else {
                return Err(ImageErrors::ImageDecoderNotIncluded(ImageFormat::Unknown));
            }
        }

        let options = encoder_options(args);

        if let Some(source) = args.value_source("out") {
            if source == CommandLine {
                for out_file in args.get_raw("out").unwrap() {
                    if let Some(ext) = Path::new(out_file).extension() {
                        if let Some(encode_type) =
                            ImageFormat::encoder_for_extension(ext.to_str().unwrap())
                        {
                            debug!("Treating {:?} as a {:?} format", out_file, encode_type);
                            workflow.formats.push(encode_type);
                        } else {
                            error!("Unknown or unsupported format {:?}", out_file)
                        }
                    } else {
                        if out_file != "-" {
                            error!("Could not determine extension from {:?}", out_file);
                        }
                    }
                }
            }
        }

        workflow.inner.advance_to_end()?;

        // write to output

        //  We support multiple format writes per invocation
        // i.e it's perfectly valid to do -o a.ppm , -o a.png
        if let Some(source) = args.value_source("out") {
            if source == CommandLine {
                for out_file in args.get_raw("out").unwrap() {
                    if out_file == "-" {
                        if let Some(cmd_format) = args.get_one::<CmdImageFormats>("output-format") {
                            // test on jpeg only
                            let mut out_file = std::io::stdout();

                            let CmdImageFormats::Format(format) = cmd_format;
                            for image in workflow.inner.images() {
                                format.encode(image, options, &mut out_file)?;
                            }
                        } else {
                            error!("You must specify the image format to be used while using output as '-` via the --output-format flag ");
                        }
                    } else {
                        //write to file
                        if let Some(ext) = Path::new(out_file).extension() {
                            for format in &workflow.formats {
                                if format.has_encoder() {
                                    for image in workflow.inner.images() {
                                        let fd = OpenOptions::new()
                                            .create(true)
                                            .write(true)
                                            .truncate(true)
                                            .open(out_file);
                                        match fd {
                                            Ok(file) => {
                                                let mut file_c = BufWriter::new(file);
                                                let start = Instant::now();
                                                let bytes =
                                                    format.encode(image, options, &mut file_c)?;
                                                let end = Instant::now();
                                                trace!(
                                                    "Took {:?} to encode {} bytes to {:?}",
                                                    end - start,
                                                    bytes,
                                                    out_file
                                                );
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Cannot encode to file, error opening {:?}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(view) = args.value_source("view") {
            if view == CommandLine {
                for image in workflow.inner.images() {
                    open_in_default_app(image);
                }
            }
        }
    }

    Ok(())
}

pub fn add_operations(args: &ArgMatches, workflow: &mut Pipeline) -> Result<(), String> {
    for id in args.ids() {
        if args.try_get_many::<clap::Id>(id.as_str()).is_ok() {
            // ignore groups
            continue;
        }

        let value_source = args
            .value_source(id.as_str())
            .expect("id came from matches");

        if value_source != clap::parser::ValueSource::CommandLine {
            // ignore things not passed via command line
            continue;
        }

        crate::cmd_parsers::operations::parse_options(workflow, id.as_str(), args)?;
        crate::cmd_parsers::filters::parse_options(workflow, id.as_str(), args)?;
    }

    Ok(())
}

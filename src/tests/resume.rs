// SPDX-License-Identifier: Unlicense

//! Tests for `crate::resume` (Phase 6).

use crate::resume::parse_resume;
use crate::Profile;

use super::{case, check, check_eq, TestResult};

const SAMPLE: &str = "Jane Q. Doe
jane.doe@example.com
+1 (555) 010-2030
linkedin.com/in/janedoe
https://github.com/janedoe
janedoe.dev

Senior Software Engineer with 7 years experience.";

pub fn run() -> Vec<TestResult> {
    vec![
        case("resume::parses_email", parses_email),
        case("resume::parses_phone", parses_phone),
        case("resume::parses_linkedin", parses_linkedin),
        case("resume::parses_github", parses_github),
        case("resume::parses_website", parses_website),
        case("resume::parses_name", parses_name),
        case("resume::raw_text_preserved_exactly", raw_text_preserved_exactly),
        case("resume::missing_fields_remain_empty_not_panic", missing_fields_remain_empty_not_panic),
        case("resume::email_requires_a_dotted_domain", email_requires_a_dotted_domain),
        case("resume::email_with_plus_tag_is_preserved", email_with_plus_tag_is_preserved),
        case("resume::email_with_dot_in_local_part", email_with_dot_in_local_part),
        case("resume::first_email_wins_when_multiple_present", first_email_wins_when_multiple_present),
        case("resume::phone_with_us_country_code_dot_format", phone_with_us_country_code_dot_format),
        case("resume::phone_dashes_only", phone_dashes_only),
        case("resume::phone_parens_only", phone_parens_only),
        case("resume::linkedin_url_with_trailing_slash", linkedin_url_with_trailing_slash),
        case("resume::github_with_repo_path_is_still_classified_as_github",
             github_with_repo_path_is_still_classified_as_github),
        case("resume::website_distinct_from_linkedin_and_github",
             website_distinct_from_linkedin_and_github),
        case("resume::empty_resume_yields_empty_profile_no_panic",
             empty_resume_yields_empty_profile_no_panic),
        case("resume::whitespace_only_resume_yields_empty_profile",
             whitespace_only_resume_yields_empty_profile),
        case("resume::name_skips_header_line_with_email", name_skips_header_line_with_email),
        case("resume::single_word_name_is_rejected", single_word_name_is_rejected),
        case("resume::very_long_first_line_is_not_a_name", very_long_first_line_is_not_a_name),
        case("resume::url_excluded_from_name_detection", url_excluded_from_name_detection),
        case("resume::raw_text_is_preserved_byte_for_byte", raw_text_is_preserved_byte_for_byte),
        case("resume::parse_is_idempotent", parse_is_idempotent),
    ]
}

fn parses_email() -> Result<(), String> {
    let p = parse_resume(SAMPLE);
    check_eq(p.email, "jane.doe@example.com".to_string(), "email")
}

fn parses_phone() -> Result<(), String> {
    let p = parse_resume(SAMPLE);
    let digits: String = p.phone.chars().filter(|c| c.is_ascii_digit()).collect();
    check_eq(digits, "15550102030".to_string(), "digit-only phone")
}

fn parses_linkedin() -> Result<(), String> {
    let p = parse_resume(SAMPLE);
    check(p.linkedin.contains("linkedin.com/in/janedoe"), "linkedin parsed")
}

fn parses_github() -> Result<(), String> {
    let p = parse_resume(SAMPLE);
    check(p.github.contains("github.com/janedoe"), "github parsed")
}

fn parses_website() -> Result<(), String> {
    let p = parse_resume(SAMPLE);
    check_eq(p.website, "janedoe.dev".to_string(), "website")
}

fn parses_name() -> Result<(), String> {
    let p = parse_resume(SAMPLE);
    check_eq(p.full_name, "Jane Q. Doe".to_string(), "full_name")
}

fn raw_text_preserved_exactly() -> Result<(), String> {
    let p = parse_resume(SAMPLE);
    check_eq(p.raw_resume_text, SAMPLE.to_string(), "raw_resume_text")
}

fn missing_fields_remain_empty_not_panic() -> Result<(), String> {
    let minimal = "Jane Doe\njane@example.com\n";
    let p = parse_resume(minimal);
    check_eq(p.email, "jane@example.com".to_string(), "email")?;
    check(p.phone.is_empty(), "phone empty")?;
    check(p.linkedin.is_empty(), "linkedin empty")?;
    check(p.github.is_empty(), "github empty")?;
    check(p.website.is_empty(), "website empty")
}

fn email_requires_a_dotted_domain() -> Result<(), String> {
    let p = parse_resume("Jane Doe\nfoo@bar\n");
    check(p.email.is_empty(), "no email match")
}

fn email_with_plus_tag_is_preserved() -> Result<(), String> {
    let p = parse_resume("Jane Doe\njane+resumes@example.com\n");
    check_eq(p.email, "jane+resumes@example.com".to_string(), "email with +tag")
}

fn email_with_dot_in_local_part() -> Result<(), String> {
    let p = parse_resume("J. Doe\nfirst.last@sub.example.com\n");
    check_eq(p.email, "first.last@sub.example.com".to_string(), "email with dot")
}

fn first_email_wins_when_multiple_present() -> Result<(), String> {
    let p = parse_resume("Jane Doe\nprimary@example.com\nbackup@example.org\n");
    check_eq(p.email, "primary@example.com".to_string(), "first email wins")
}

fn phone_with_us_country_code_dot_format() -> Result<(), String> {
    let p = parse_resume("Jane Doe\nj@e.com\n+1.555.010.2030\n");
    let digits: String = p.phone.chars().filter(|c| c.is_ascii_digit()).collect();
    check_eq(digits, "15550102030".to_string(), "us country code")
}

fn phone_dashes_only() -> Result<(), String> {
    let p = parse_resume("Jane Doe\nj@e.com\n555-010-2030\n");
    let digits: String = p.phone.chars().filter(|c| c.is_ascii_digit()).collect();
    check_eq(digits, "5550102030".to_string(), "dashes-only phone")
}

fn phone_parens_only() -> Result<(), String> {
    let p = parse_resume("Jane Doe\nj@e.com\n(555) 010 2030\n");
    let digits: String = p.phone.chars().filter(|c| c.is_ascii_digit()).collect();
    check_eq(digits, "5550102030".to_string(), "parens-only phone")
}

fn linkedin_url_with_trailing_slash() -> Result<(), String> {
    let p = parse_resume("Jane Doe\nj@e.com\nhttps://linkedin.com/in/janedoe/\n");
    check(p.linkedin.contains("linkedin.com/in/janedoe"), "linkedin trailing slash")
}

fn github_with_repo_path_is_still_classified_as_github() -> Result<(), String> {
    let p = parse_resume("Jane Doe\nj@e.com\ngithub.com/janedoe/atsisbroken\n");
    check(p.github.contains("github.com/janedoe"), "github with repo path")
}

fn website_distinct_from_linkedin_and_github() -> Result<(), String> {
    let p = parse_resume(
        "Jane Doe\nj@e.com\nlinkedin.com/in/janedoe\ngithub.com/janedoe\nportfolio.dev\n",
    );
    check_eq(p.website.as_str(), "portfolio.dev", "website")?;
    check(!p.website.contains("linkedin"), "website not linkedin")?;
    check(!p.website.contains("github"), "website not github")
}

fn empty_resume_yields_empty_profile_no_panic() -> Result<(), String> {
    let p = parse_resume("");
    check(p.full_name.is_empty(), "full_name empty")?;
    check(p.email.is_empty(), "email empty")?;
    check(p.phone.is_empty(), "phone empty")?;
    check_eq(p.raw_resume_text, "".to_string(), "raw_resume_text empty")
}

fn whitespace_only_resume_yields_empty_profile() -> Result<(), String> {
    let p = parse_resume("\n\n   \n\t\n");
    check(p.full_name.is_empty(), "full_name empty")?;
    check(p.email.is_empty(), "email empty")
}

fn name_skips_header_line_with_email() -> Result<(), String> {
    let p = parse_resume("jane@example.com | linkedin.com/in/janedoe\nJane Q. Doe\n");
    check_eq(p.full_name, "Jane Q. Doe".to_string(), "name from second line")
}

fn single_word_name_is_rejected() -> Result<(), String> {
    let p = parse_resume("Jane\njane@example.com\n");
    check_eq(p.full_name, "".to_string(), "single word rejected")
}

fn very_long_first_line_is_not_a_name() -> Result<(), String> {
    let p = parse_resume(
        "Senior Software Engineer with deep experience in distributed systems\nJane Doe\njane@example.com\n",
    );
    check_eq(p.full_name, "Jane Doe".to_string(), "long line skipped")
}

fn url_excluded_from_name_detection() -> Result<(), String> {
    let p = parse_resume("https://janedoe.dev\nJane Doe\njane@example.com\n");
    check_eq(p.full_name, "Jane Doe".to_string(), "url skipped for name")
}

fn raw_text_is_preserved_byte_for_byte() -> Result<(), String> {
    let weird = "Jane Doe\r\njane@e.com\r\n\nextra\n";
    let p = parse_resume(weird);
    check_eq(p.raw_resume_text, weird.to_string(), "raw bytes preserved")
}

fn parse_is_idempotent() -> Result<(), String> {
    let s = "Jane Doe\njane@example.com\n+1-555-010-2030\n";
    let a: Profile = parse_resume(s);
    let b: Profile = parse_resume(s);
    check_eq(a, b, "parse is idempotent")
}

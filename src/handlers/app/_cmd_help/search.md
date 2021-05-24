/s "" @"" &"" - # $ s d c u



<!-- Search for items by conditions.
/s {condition} {condition} ...


EXAMPLES OF A CONDITION -->

<!-- "TIT LE" <!-- title contains TIT and LE -->
<!-- @"USER NAME" <!-- username contains USER and NAME-->
<!-- &"URL LINK" <!-- url link contains URL and LINK -->
<!-- -a <!-- archived -->
<!-- -!s <!-- not starred -->
<!-- -l!r <!-- leaf && not root -->
<!-- 333<#<777 <!-- #333's successor && #777's predecessor -->
<!-- .5<$<24 <!-- 0.5h <= weight <= 24.0h -->
<!-- s<15: <!-- startable <= 15:00 today -->
<!-- /12/<d <!-- December 1st this year <= deadline -->
<!-- c <!-- created_at anytime -->
<!-- 2021//<u<//30T6: <!-- New Year's Day 2021 <= updated_at <= 6:00 on 30th of this month-->
<!-- r"REGEX_TITLE" <!-- title matches REGEX_TITLE -->
<!-- @r"REGEX_USER" <!-- username matches REGEX_USER -->
<!-- &r"REGEX_URL" <!-- url link matches REGEX_URL -->
<!-- #"DOUBLE "QUOTED" TITLE"# <!-- title contains DOUBLE, "QUOTED" and TITLE -->
<!-- r##"REGEX_SHARP#"QUOTED"#TITLE"## <!-- title matches REGEX_SHARP#"QUOTED"#TITLE -->

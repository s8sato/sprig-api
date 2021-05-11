/s "" - # $ s d c u @"" &""



<!-- Search for tasks by conditions.
/s {condition} {condition} ...


EXAMPLES OF A CONDITION -->

<!-- "TIT LE" <!-- title contains TIT and LE -->
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
<!-- #"DOUBLE "QUOTED" TITLE"# <!-- title contains DOUBLE, "QUOTED" and TITLE -->
<!-- r##"REGEX_SHARP#"QUOTED"#TITLE"## <!-- title matches REGEX_SHARP#"QUOTED"#TITLE -->
<!-- @"USER" <!-- username contains USER -->
<!-- @r"REGEX_USER" <!-- username matches REGEX_USER -->
<!-- &"URL" <!-- url link contains URL -->
<!-- &r"REGEX_URL" <!-- url link matches REGEX_URL -->

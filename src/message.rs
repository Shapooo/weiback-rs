use std::ops::RangeInclusive;

pub enum Message {
    DownloadMeta(RangeInclusive<u32>),
    DownloadWithPic(RangeInclusive<u32>),
    ExportFromNet(RangeInclusive<u32>),
    ExportFromLocal(RangeInclusive<u32>, bool),
}
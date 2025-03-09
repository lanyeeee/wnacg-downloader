use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ImgList(pub Vec<ImgInImgList>);
impl Deref for ImgList {
    type Target = Vec<ImgInImgList>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ImgList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl IntoIterator for ImgList {
    type Item = ImgInImgList;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::module_name_repetitions)]
pub struct ImgInImgList {
    /// 图片标题([01]、[001]，根据漫画总页数确定)
    pub caption: String,
    /// 图片url(//img5.wnimg.ru/data/2826/33/01.jpg，缺https:前缀)  
    /// 最后一张图片为/themes/weitu/images/bg/shoucang.jpg，记得过滤
    pub url: String,
}

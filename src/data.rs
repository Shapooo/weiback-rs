use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

pub type Post = Value;

#[derive(Deserialize, Debug, Clone)]
pub struct Posts {
    pub data: Vec<Value>,
}

impl Serialize for Posts {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl Posts {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }

    pub fn append(&mut self, mut appendee: Posts) {
        self.data.append(&mut appendee.data);
    }
}

impl IntoIterator for Posts {
    type Item = Post;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

pub type User = Value;

#[derive(Deserialize, Debug, Clone)]
pub struct LongText {
    ok: u8,
    http_code: u8,
    data: LongTextContent,
}

#[derive(Deserialize, Debug, Clone)]
struct LongTextContent {
    #[serde(rename = "longTextContent")]
    pub long_text_content: String,
}

impl LongText {
    pub fn get_content(self) -> Result<String> {
        if self.ok == 1 && self.http_code == 200 {
            Ok(self.data.long_text_content)
        } else {
            Err(Error::ResourceGetFailed(
                "returned long text status is not ok",
            ))
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct FavTag {
    pub fav_total_num: u64,
    pub ok: u8,
}

#[cfg(test)]
mod data_test {
    use super::{LongText, Post, Posts};

    #[test]
    fn parse_post() {
        let txt = include_str!("../res/one.json");
        serde_json::from_str::<Post>(txt).unwrap();
    }

    #[test]
    fn parse_posts() {
        let txt = include_str!("../res/full.json");
        serde_json::from_str::<Posts>(txt).unwrap();
    }

    #[test]
    fn parse_long_text() {
        let txt = r#"{"ok":1,"http_code":200,"data":{"longTextContent":"5月27日台大毕业典礼上NVIDIA公司创始人黄仁勋演讲全文：\n\n你们所处的年代很复杂，却也是你们的机会。\n\n当我从俄勒冈州立大学毕业时，世界还比较简单。电视还很大一台，没有无线电视跟MTV、没有手机。那是1994年，IBM个人电脑跟MAC麦金塔开始了个人电脑革命。开始日后芯片与运算程式的发展。\n\n你们正处在的世界更复杂，面临着地缘政治、社会和环境上的变化和挑战，被科技包围着。我们处于一个永远连接和沉浸的数据世界，与现实世界平行存在。在40年前，当电脑产业创造了家用PC，持续研究AI技术，我们的运算程式驾驶着汽车、或研读X光片影像。AI为电脑自动化开启了大门，其服务涵盖了世界最大的兆级产业：健康照护、金融服务、运输与制造产业。\n\nAI为我们带来了巨大的机遇，反应敏捷的企业将利用AI技术提升竞争力，而未能善用AI的企业将面临衰退。很多企业家，包含今天在场的许多人，未来将会开创新公司。\n\n如同过去的每个计算机时代能创造新的产业，AI也创造了以前不存在的新工作机会，像是：数据工程师、咏唱工程师、AI工厂操作员和AI安全工程师等，这些工作以前从未存在过。\n\n自动化工作将淘汰一些工作，并且毫无疑问的，AI会改变每一个工作，大幅加强设计师、艺术家、销售和制造计划者的工作表现。就像在你们之前的每个世代，拥抱科技以获得成功。每个公司与你必须学会利用AI的优势，在AI的帮助下做出惊人成就。\n\n有些人担心AI可能会抢走你的工作，有些人可能会担心AI发展出自我意志。我们正处于一个新领域的开始，就像个人电脑、网路、移动设备与云端技术一样。但是AI的影响更为根本，每个运算层面都会被重新改写。它改变了我们撰写软件、执行软件的方式。\n\n从各方面来看，这是电脑产业的再生契机。你们正是这个产业的重要基石。在下个十年，我们的产业将使用新型AI电脑取代价值上兆美元的传统电脑。\n\n我的旅程始于你们40年之前，1984年是一个完美的毕业年份，我预测2023年也将如此。我能告诉你什么呢？今天是迄今为止你们最成功的一天，你们从台大毕业了，我也曾经成功过。在我创办了Nvidia前，我经历过失败，而且是大失败，说起来令人耻辱和尴尬，甚至几乎让我们走向毁灭。让我给你们讲3个故事，这些故事定义了Nvidia今天的样貌。\n\n坦诚面对错误，谦卑寻求帮助 是聪明、成功人士最难学会的\n\n我们创办Nvidia是为了创造加速运算技术。我们的第一个应用是用于个人电脑游戏的3D图形，我们发明了一种非传统的前向纹理处理技术，而且成本相对低廉。我们赢得了与SEGA建造游戏主机的合约。这吸引了游戏开发商用我们的平台开发游戏，并提供我们公司资金。\n\n但经过了一年的开发期程，我们意识到我们设计的架构是错误策略，从技术端来看是不合格的。而与此同时，微软即将宣布基于反向纹理映射和三角形的Windows 95 Direct3D。这代表如果我们完成了SEGA的游戏机，我们将会创造出与Windows不相容的产品；但如果我们不完成这个合约，我们就会破产。无论如何，我们都会面临倒闭的命运。\n\n我联络了SEGA执行长，向他解释我们的发明是错误的，我们无法完成合约以及游戏主机，并建议SEGA寻找其他合作伙伴。我对他说：‘我们必须停下来。’\n\n但我需要SEGA全额支付我们的费用，否则Nvidia将无法继续经营。\n\n我很难为情的向SEGA执行长提出这个要求，但令我惊讶的是，他同意了。他的理解和慷慨让我们多活了3个月，在那段时间，我们建造了Riva 128，就在我们差点没钱时，Riva 128震撼了新兴的3D市场，让我们开始受到关注，也拯救了公司营运。\n\n市场对我们的芯片需求旺盛，让我从4岁离开台湾后又回到了台湾。我与台积电的张忠谋先生会面，并开始一段持续25年的合作关系。\n\n我们坦诚面对错误、谦卑的寻求帮助，拯救Nvidia的存续。这些特质对于像你们这样最聪明、最成功的人而言，是最难养成的。\n\n追求愿景的艰苦过程 塑造我们的品格\n\n在2007年，我们宣布了CUDA GPU加速计算技术，我们的期望是让CUDA成为一个程式设计模型，在科学运算、物理模拟到图像处理方面，都能提升应用程式的效能。\n\n创建一个全新的运算模型非常困难，且在历史上实属罕见。自从IBM System 360以来，CPU的运算模型已经成为标准已有60年的时间。CUDA需要开发人员撰写应用程式，并展示GPU的优势；开发人员需要一个大型的使用者基础；大型的CUDA使用者基础，需要市场上有人购买新的应用程式。\n\n因此，为了解决先有鸡还是先有蛋的问题。\n\n我们利用我们的游戏显卡GPU GeForce，它已经拥有庞大的游戏市场，以建立使用者基础。但CUDA的成本非常高，Nvidia的利润在多年来遭受巨大的打击，我们的市值仅仅维持在10亿美元上下。我们多年的低迷表现，让股东们对CUDA持怀疑态度，并希望我们专注于提高盈利能力。\n\n但我们坚持下来，我们相信加速运算的时代将会到来，我们创建了一个名为GTC的会议，并在全球不辞辛劳的推广CUDA技术。\n\n然后CT重建、分子动力学、粒子物理学、流体动力学和图像处理等应用程式开始大量出现，我们的开发人员撰写算法，并加快了芯片运算速度。\n\n2012年，AI研究人员探索了CUDA，著名的AlexNet在我们的GPU GTX 580上进行了训练，开启AI的大爆炸，幸运的是，我们意识到了深度学习的潜力，我们冒着一切风险去追求深度学习。多年后，AI革命开始了，Nvidia成为了推动引擎。我们为AI发明了CUDA，这个旅程锻造了我们的品格，承受痛苦和苦难，是在追求愿景的路上必经之痛。\n\n学会放弃 是迈向成功的核心关键\n\n再讲一个故事，在2010年，Google将Android系统打造成出色图形的平台，而手机行业也有调制解调器的芯片公司。Nvidia优秀的运算能力，让Nvidia成为Android系统良好的合作伙伴。我们取得成功、股价飙升，但竞争对手也很快就涌入，调制解调器制造商们也在学习如何生产运算芯片，而我们却在学习调制解调器。\n\n因为手机市场庞大，我们能抢占市占率。然而，我们却做出艰难的决定，放弃这块市场。因为Nvidia的使命，是创造出能解决‘普通电脑解决的问题’的电脑，我们应该专注在愿景上，发挥我们的独特贡献。\n\n我们的放弃获得了回报，我们创造了一个新的市场——机器人技术，拥有神经网路处理器和运行AI算法的安全架构。\n\n当时，这还是个看不见规模的市场。从巨大的手机市场撤退，再创造一个不知道市场规模的机器人市场。然而，现在的我们拥有数十亿美元的自动驾驶、机器人技术的事业，也开创一个新的产业。\n\n‘撤退’对像你们如此聪明且成功的人来说并不容易。然而，战略性的撤退、牺牲、决定放弃什么是成功的核心，非常关键的核心。\n\n跑吧！无论如何都要保持着奔跑\n\n2023年毕业的同学们，你们即将进入一个正在经历巨大变革的世界，就像我毕业时遇到个人电脑和芯片革命时一样，你们正处于AI的起跑线上。每个行业都将被革命、重生，为新思想做好准备——你们的思想。\n\n在40年的时间里，我们创造了个人电脑、网路、移动设备、云端技术。现在的AI时代，你们将创造什么？\n\n无论是什么，像我们一样全力以赴去追求它，跑吧！不要慢慢走。\n\n不论是为了食物而奔跑，或不被他人当做食物而奔跑。你往往无法知道自己正处在哪一种情况，但无论如何，都要保持奔跑。\n\n在你的旅程中，带上一些我犯过的错、有过的经验。希望你们能谦卑的面对失败，承认错误并寻求帮助。你们将承受实现梦想所需的痛苦和苦难，并做出牺牲，致力于有意义的生活，冲刺你们人生的事业。"}}"#;
        serde_json::from_str::<LongText>(txt).unwrap();
    }
}

use async_recursion::async_recursion;
use rug::Integer;

use super::{resolve_dice, RRVal, RVal};

#[derive(Debug, Clone, PartialEq)]
pub enum LazyValue {
    Int(Integer),
    Float(f64),
    Array(Vec<LazyValue>),
    LazyDice {
        num: u32,
        sides: Vec<RRVal>,
        lowest_idx: u32,
        highest_idx: u32,
        explode: Vec<RRVal>,
    },
}

impl LazyValue {
    pub async fn add(self, rhs: LazyValue) -> LazyValue {
        self.deep_resolve()
            .await
            .add(rhs.deep_resolve().await)
            .await
            .into()
    }

    pub async fn sub(self, rhs: LazyValue) -> LazyValue {
        self.deep_resolve()
            .await
            .sub(rhs.deep_resolve().await)
            .await
            .into()
    }

    pub async fn mul(self, rhs: LazyValue) -> LazyValue {
        self.deep_resolve()
            .await
            .mul(rhs.deep_resolve().await)
            .await
            .into()
    }

    pub async fn div(self, rhs: LazyValue) -> LazyValue {
        self.deep_resolve()
            .await
            .div(rhs.deep_resolve().await)
            .await
            .into()
    }

    #[async_recursion]
    pub async fn neg(self) -> LazyValue {
        self.resolve().await.neg().await.into()
    }

    #[async_recursion]
    pub async fn display(&self, s: &mut String) {
        self.clone().resolve().await.display(s).await;
    }

    pub async fn resolve(self) -> RVal {
        match self {
            LazyValue::Int(n) => RVal::Int(n),
            LazyValue::Float(f) => RVal::Float(f),
            LazyValue::Array(a) => RVal::Array(a),
            LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => resolve_dice(num, sides, lowest_idx, highest_idx, explode).await.into(),
        }
    }

    pub async fn deep_resolve(self) -> RRVal {
        match self {
            LazyValue::Int(n) => RRVal::Int(n),
            LazyValue::Float(f) => RRVal::Float(f),
            LazyValue::Array(a) => RRVal::Array(RRVal::deep_resolve_vec(a).await),
            LazyValue::LazyDice {
                num,
                sides,
                lowest_idx,
                highest_idx,
                explode,
            } => resolve_dice(num, sides, lowest_idx, highest_idx, explode).await,
        }
    }

    pub async fn into_i32(self) -> Result<i32, String> {
        self.resolve().await.into_i32()
    }

    #[async_recursion]
    pub async fn op_eq(self, other: LazyValue) -> LazyValue {
        self.resolve()
            .await
            .op_eq(other.resolve().await)
            .await
            .into()
    }

    #[async_recursion]
    pub async fn op_or(self, other: LazyValue) -> LazyValue {
        self.resolve()
            .await
            .op_or(other.resolve().await)
            .await
            .into()
    }
}

impl<T> From<Vec<T>> for LazyValue
where
    T: Into<LazyValue>,
{
    fn from(value: Vec<T>) -> Self {
        LazyValue::Array(value.into_iter().map(|x| x.into()).collect())
    }
}

#[tokio::test]
async fn display_test() {
    macro_rules! eq {
        ($x:expr, $y:expr) => {
            {
                let mut s = String::new();
                LazyValue::from($x).display(&mut s).await;
                assert_eq!(s, $y);
            }
        }
    }

    eq!(vec![2], "(,2)");
    eq!(vec![1,2,3],"(1, 2, 3)");
    eq!(vec![vec![1,2],vec![3],vec![4,5]],"((1, 2), (,3), (4, 5))");
    eq!(Vec::<i32>::new(), "()");
}

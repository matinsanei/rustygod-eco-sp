//! gRPC services over `rustygod_db::commerce`: discount, shipping,
//! menu, page, account, channel, tax, warehouse.
//! (Gift cards live in the dedicated `GiftCardService`, `service_giftcard`.)

use rustygod_db::commerce;
use rustygod_proto::commerce::{
    account_service_server::AccountService, channel_service_server::ChannelService,
    discount_service_server::DiscountService,
    menu_service_server::MenuService, page_service_server::PageService,
    shipping_service_server::ShippingService, tax_service_server::TaxService,
    warehouse_service_server::WarehouseService, *,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};

impl WarehouseServiceImpl {
    fn wh_info(w: &rustygod_db::entities::warehouse_warehouse::Model) -> WarehouseInfo {
        WarehouseInfo {
            id: w.id.to_string(),
            name: w.name.clone(),
            slug: w.slug.clone(),
            email: w.email.clone(),
        }
    }

    fn wh_err(e: rustygod_db::DbError) -> rustygod_proto::common::Error {
        err("WAREHOUSE_ERROR", e.to_string())
    }
}

impl ChannelServiceImpl {
    fn ch_err(e: rustygod_db::DbError) -> rustygod_proto::common::Error {
        err("CHANNEL_ERROR", e.to_string())
    }
}

impl AccountServiceImpl {
    fn group_info(g: &rustygod_db::groups::GroupView) -> GroupInfo {
        GroupInfo {
            id: g.id.to_string(),
            name: g.name.clone(),
            permissions: g.permissions.clone(),
            member_ids: g.member_ids.iter().map(|i| i.to_string()).collect(),
        }
    }

    fn group_err(e: rustygod_db::DbError) -> rustygod_proto::common::Error {
        err("GROUP_ERROR", e.to_string())
    }

    async fn group_members(
        &self,
        request: Request<GroupMembersRequest>,
        add: bool,
    ) -> Result<Response<GroupResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_STAFF).await?;
        let r = request.into_inner();
        let id: i32 = r.id.parse().map_err(|_| Status::invalid_argument("id must be an integer"))?;
        let mut uids = Vec::with_capacity(r.user_ids.len());
        for u in &r.user_ids {
            uids.push(u.parse::<i32>().map_err(|_| Status::invalid_argument("user_ids must be integers"))?);
        }
        let out = if add {
            rustygod_db::groups::add_members(db, id, &uids).await
        } else {
            rustygod_db::groups::remove_members(db, id, &uids).await
        };
        match out {
            Ok(g) => Ok(Response::new(GroupResponse {
                group: Some(Self::group_info(&g)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GroupResponse {
                group: None,
                errors: vec![Self::group_err(e)],
            })),
        }
    }

    async fn group_permissions(
        &self,
        request: Request<GroupPermissionsRequest>,
        grant: bool,
    ) -> Result<Response<GroupResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_STAFF).await?;
        let r = request.into_inner();
        let id: i32 = r.id.parse().map_err(|_| Status::invalid_argument("id must be an integer"))?;
        let out = if grant {
            rustygod_db::groups::grant_permissions(db, id, &r.codenames).await
        } else {
            rustygod_db::groups::revoke_permissions(db, id, &r.codenames).await
        };
        match out {
            Ok(g) => Ok(Response::new(GroupResponse {
                group: Some(Self::group_info(&g)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GroupResponse {
                group: None,
                errors: vec![Self::group_err(e)],
            })),
        }
    }
}

fn err(code: &str, message: String) -> rustygod_proto::common::Error {
    rustygod_proto::common::Error {
        code: code.into(),
        message,
        field: String::new(),
    }
}

fn unavailable<T>() -> Result<Response<T>, Status> {
    Err(Status::unavailable("postgres unavailable in offline mode"))
}

macro_rules! svc {
    ($name:ident) => {
        pub struct $name {
            db: Option<DatabaseConnection>,
        }
        impl $name {
            pub fn new(db: Option<DatabaseConnection>) -> Self {
                Self { db }
            }
            fn db(&self) -> Result<&DatabaseConnection, Status> {
                self.db.as_ref().ok_or_else(|| Status::unavailable("postgres unavailable"))
            }
        }
    };
}

svc!(DiscountServiceImpl);
svc!(ShippingServiceImpl);
svc!(MenuServiceImpl);
svc!(PageServiceImpl);
svc!(AccountServiceImpl);
svc!(ChannelServiceImpl);
svc!(TaxServiceImpl);
svc!(WarehouseServiceImpl);

fn channel_of(req: &str) -> &str {
    if req.is_empty() { "default-channel" } else { req }
}

#[tonic::async_trait]
impl DiscountService for DiscountServiceImpl {
    async fn validate_voucher(
        &self,
        request: Request<ValidateVoucherRequest>,
    ) -> Result<Response<ValidateVoucherResponse>, Status> {
        let req = request.into_inner();
        match commerce::validate_voucher(self.db()?, &req.code, channel_of(&req.channel)).await {
            Ok(v) => Ok(Response::new(ValidateVoucherResponse {
                voucher: Some(VoucherInfo {
                    code: v.code,
                    r#type: v.voucher_type,
                    discount_value_type: v.discount_value_type,
                    currency: v.currency,
                    discount_value: v.discount_value.to_string(),
                    min_spent_amount: v.min_spent.map(|m| m.to_string()).unwrap_or_default(),
                    usage_limit: v.usage_limit.unwrap_or(-1),
                    used: v.used,
                }),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(ValidateVoucherResponse {
                voucher: None,
                errors: vec![err("INVALID", e.to_string())],
            })),
        }
    }

    async fn list_promotions(
        &self,
        request: Request<ListPromotionsRequest>,
    ) -> Result<Response<ListPromotionsResponse>, Status> {
        let req = request.into_inner();
        let promos = commerce::list_promotions(self.db()?, channel_of(&req.channel))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListPromotionsResponse {
            promotions: promos
                .into_iter()
                .map(|p| PromotionInfo {
                    id: p.id,
                    name: p.name,
                    r#type: p.promotion_type,
                    rules: p
                        .rules
                        .into_iter()
                        .map(|r| PromotionRuleInfo {
                            id: r.id,
                            name: r.name,
                            reward_type: r.reward_type,
                            reward_value_type: r.reward_value_type,
                            reward_value: r.reward_value,
                        })
                        .collect(),
                })
                .collect(),
        }))
    }
}

#[tonic::async_trait]
impl ShippingService for ShippingServiceImpl {
    async fn list_shipping_methods(
        &self,
        request: Request<ListShippingMethodsRequest>,
    ) -> Result<Response<ListShippingMethodsResponse>, Status> {
        let req = request.into_inner();
        let methods = commerce::list_shipping_methods(self.db()?, channel_of(&req.channel))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListShippingMethodsResponse {
            methods: methods
                .into_iter()
                .map(|m| ShippingMethodInfo {
                    id: m.id.to_string(),
                    name: m.name,
                    r#type: m.method_type,
                    currency: m.currency,
                    price_amount: m.price_amount.to_string(),
                    minimum_order_price_amount: m
                        .min_order_price
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                })
                .collect(),
        }))
    }
}

fn menu_item_to_proto(m: &commerce::MenuItemView) -> MenuItemInfo {    MenuItemInfo {
        id: m.id.to_string(),
        name: m.name.clone(),
        url: m.url.clone(),
        level: m.level,
        children: m.children.iter().map(menu_item_to_proto).collect(),
    }
}

#[tonic::async_trait]
impl MenuService for MenuServiceImpl {
    async fn get_menu(
        &self,
        request: Request<GetMenuRequest>,
    ) -> Result<Response<GetMenuResponse>, Status> {
        match commerce::get_menu(self.db()?, &request.into_inner().slug)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(m) => Ok(Response::new(GetMenuResponse {
                menu: Some(MenuInfo {
                    id: m.id.to_string(),
                    name: m.name,
                    slug: m.slug,
                    items: m.items.iter().map(menu_item_to_proto).collect(),
                }),
                errors: vec![],
            })),
            None => Ok(Response::new(GetMenuResponse {
                menu: None,
                errors: vec![err("NOT_FOUND", "menu not found".into())],
            })),
        }
    }
}

fn page_to_proto(p: &commerce::PageView) -> PageInfo {
    PageInfo {
        id: p.id.to_string(),
        slug: p.slug.clone(),
        title: p.title.clone(),
        is_published: p.is_published,
    }
}

#[tonic::async_trait]
impl PageService for PageServiceImpl {
    async fn get_page(
        &self,
        request: Request<GetPageRequest>,
    ) -> Result<Response<GetPageResponse>, Status> {
        match commerce::get_page(self.db()?, &request.into_inner().slug)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(p) => Ok(Response::new(GetPageResponse {
                page: Some(page_to_proto(&p)),
                errors: vec![],
            })),
            None => Ok(Response::new(GetPageResponse {
                page: None,
                errors: vec![err("NOT_FOUND", "page not found".into())],
            })),
        }
    }

    async fn list_pages(
        &self,
        _request: Request<ListPagesRequest>,
    ) -> Result<Response<ListPagesResponse>, Status> {
        let pages = commerce::list_pages(self.db()?)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListPagesResponse {
            pages: pages.iter().map(page_to_proto).collect(),
        }))
    }
}

#[tonic::async_trait]
impl AccountService for AccountServiceImpl {
    async fn get_customer(
        &self,
        request: Request<GetCustomerRequest>,
    ) -> Result<Response<GetCustomerResponse>, Status> {
        match commerce::get_customer(self.db()?, &request.into_inner().email)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(c) => Ok(Response::new(GetCustomerResponse {
                customer: Some(CustomerInfo {
                    id: c.id.to_string(),
                    email: c.email,
                    first_name: c.first_name,
                    last_name: c.last_name,
                    is_active: c.is_active,
                }),
                errors: vec![],
            })),
            None => Ok(Response::new(GetCustomerResponse {
                customer: None,
                errors: vec![err("NOT_FOUND", "customer not found".into())],
            })),
        }
    }

    async fn create_address(
        &self,
        request: Request<AddressInput>,
    ) -> Result<Response<CreateAddressResponse>, Status> {
        let req = request.into_inner();
        let id = commerce::create_address(
            self.db()?,
            &commerce::NewAddress {
                first_name: req.first_name,
                last_name: req.last_name,
                street_address_1: req.street_address_1,
                city: req.city,
                postal_code: req.postal_code,
                country: req.country,
                phone: req.phone,
            },
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(CreateAddressResponse {
            id: id.to_string(),
            errors: vec![],
        }))
    }

    async fn create_group(
        &self,
        request: Request<CreateGroupRequest>,
    ) -> Result<Response<GroupResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_STAFF).await?;
        let r = request.into_inner();
        match rustygod_db::groups::create_group(db, &r.name, &r.permissions).await {
            Ok(g) => Ok(Response::new(GroupResponse {
                group: Some(Self::group_info(&g)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GroupResponse {
                group: None,
                errors: vec![Self::group_err(e)],
            })),
        }
    }

    async fn list_groups(
        &self,
        request: Request<ListGroupsRequest>,
    ) -> Result<Response<ListGroupsResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_STAFF).await?;
        let _ = request;
        match rustygod_db::groups::list_groups(db).await {
            Ok(gs) => Ok(Response::new(ListGroupsResponse {
                groups: gs.iter().map(Self::group_info).collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn rename_group(
        &self,
        request: Request<RenameGroupRequest>,
    ) -> Result<Response<GroupResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_STAFF).await?;
        let r = request.into_inner();
        let id: i32 = r.id.parse().map_err(|_| Status::invalid_argument("id must be an integer"))?;
        match rustygod_db::groups::rename_group(db, id, &r.name).await {
            Ok(g) => Ok(Response::new(GroupResponse {
                group: Some(Self::group_info(&g)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(GroupResponse {
                group: None,
                errors: vec![Self::group_err(e)],
            })),
        }
    }

    async fn delete_group(
        &self,
        request: Request<DeleteGroupRequest>,
    ) -> Result<Response<DeleteGroupResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_STAFF).await?;
        let id: i32 = request.into_inner().id.parse().map_err(|_| Status::invalid_argument("id must be an integer"))?;
        match rustygod_db::groups::delete_group(db, id).await {
            Ok(()) => Ok(Response::new(DeleteGroupResponse { ok: true, errors: vec![] })),
            Err(e) => Ok(Response::new(DeleteGroupResponse {
                ok: false,
                errors: vec![Self::group_err(e)],
            })),
        }
    }

    async fn add_group_members(
        &self,
        request: Request<GroupMembersRequest>,
    ) -> Result<Response<GroupResponse>, Status> {
        self.group_members(request, true).await
    }

    async fn remove_group_members(
        &self,
        request: Request<GroupMembersRequest>,
    ) -> Result<Response<GroupResponse>, Status> {
        self.group_members(request, false).await
    }

    async fn grant_group_permissions(
        &self,
        request: Request<GroupPermissionsRequest>,
    ) -> Result<Response<GroupResponse>, Status> {
        self.group_permissions(request, true).await
    }

    async fn revoke_group_permissions(
        &self,
        request: Request<GroupPermissionsRequest>,
    ) -> Result<Response<GroupResponse>, Status> {
        self.group_permissions(request, false).await
    }
}

fn channel_to_proto(c: &commerce::ChannelView) -> ChannelInfo {
    ChannelInfo {
        id: c.id.to_string(),
        slug: c.slug.clone(),
        currency_code: c.currency_code.clone(),
        is_active: c.is_active,
    }
}

#[tonic::async_trait]
impl ChannelService for ChannelServiceImpl {
    async fn list_channels(
        &self,
        _request: Request<ListChannelsRequest>,
    ) -> Result<Response<ListChannelsResponse>, Status> {
        let channels = commerce::list_channels(self.db()?)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListChannelsResponse {
            channels: channels.iter().map(channel_to_proto).collect(),
        }))
    }

    async fn get_channel(
        &self,
        request: Request<GetChannelRequest>,
    ) -> Result<Response<GetChannelResponse>, Status> {
        let slug = request.into_inner().slug;
        let channels = commerce::list_channels(self.db()?)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        match channels.into_iter().find(|c| c.slug == slug) {
            Some(c) => Ok(Response::new(GetChannelResponse {
                channel: Some(channel_to_proto(&c)),
                errors: vec![],
            })),
            None => Ok(Response::new(GetChannelResponse {
                channel: None,
                errors: vec![err("NOT_FOUND", "channel not found".into())],
            })),
        }
    }

    async fn create_channel(
        &self,
        request: Request<CreateChannelRequest>,
    ) -> Result<Response<ChannelResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_CHANNELS).await?;
        let r = request.into_inner();
        match rustygod_db::channels::create_channel(
            db,
            rustygod_db::channels::NewChannel {
                name: r.name,
                slug: r.slug,
                currency_code: r.currency_code,
                default_country: r.default_country,
                allocation_strategy: r.allocation_strategy,
            },
        )
        .await
        {
            Ok(c) => Ok(Response::new(ChannelResponse {
                channel: Some(ChannelInfo {
                    id: c.id.to_string(),
                    slug: c.slug,
                    currency_code: c.currency_code,
                    is_active: c.is_active,
                }),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(ChannelResponse {
                channel: None,
                errors: vec![Self::ch_err(e)],
            })),
        }
    }

    async fn update_channel(
        &self,
        request: Request<UpdateChannelRequest>,
    ) -> Result<Response<ChannelResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_CHANNELS).await?;
        let r = request.into_inner();
        match rustygod_db::channels::update_channel(
            db,
            &r.slug,
            rustygod_db::channels::ChannelPatch {
                name: (!r.name.is_empty()).then_some(r.name),
                is_active: r.set_active.then_some(r.is_active),
                default_country: (!r.default_country.is_empty()).then_some(r.default_country),
                allocation_strategy: (!r.allocation_strategy.is_empty())
                    .then_some(r.allocation_strategy),
                auto_confirm: r.set_auto_confirm.then_some(r.auto_confirm),
            },
        )
        .await
        {
            Ok(c) => Ok(Response::new(ChannelResponse {
                channel: Some(ChannelInfo {
                    id: c.id.to_string(),
                    slug: c.slug,
                    currency_code: c.currency_code,
                    is_active: c.is_active,
                }),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(ChannelResponse {
                channel: None,
                errors: vec![Self::ch_err(e)],
            })),
        }
    }

    async fn delete_channel(
        &self,
        request: Request<DeleteChannelRequest>,
    ) -> Result<Response<DeleteChannelResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_CHANNELS).await?;
        match rustygod_db::channels::delete_channel(db, &request.into_inner().slug).await {
            Ok(()) => Ok(Response::new(DeleteChannelResponse { ok: true, errors: vec![] })),
            Err(e) => Ok(Response::new(DeleteChannelResponse {
                ok: false,
                errors: vec![Self::ch_err(e)],
            })),
        }
    }

    async fn set_product_listing(
        &self,
        request: Request<SetProductListingRequest>,
    ) -> Result<Response<SetProductListingResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_CHANNELS).await?;
        let r = request.into_inner();
        let pid: i32 = r.product_id.parse().map_err(|_| Status::invalid_argument("product_id must be an integer"))?;
        let channel = channel_of(&r.channel).to_string();
        match rustygod_db::channels::set_product_listing(
            db,
            &channel,
            pid,
            r.is_published,
            r.visible_in_listings,
        )
        .await
        {
            Ok(()) => Ok(Response::new(SetProductListingResponse { errors: vec![] })),
            Err(e) => Ok(Response::new(SetProductListingResponse {
                errors: vec![Self::ch_err(e)],
            })),
        }
    }

    async fn set_variant_price(
        &self,
        request: Request<SetVariantPriceRequest>,
    ) -> Result<Response<SetVariantPriceResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_CHANNELS).await?;
        let r = request.into_inner();
        let vid: i32 = r.variant_id.parse().map_err(|_| Status::invalid_argument("variant_id must be an integer"))?;
        let price = if r.price.is_empty() {
            None
        } else {
            Some(r.price.parse::<rust_decimal::Decimal>().map_err(|_| Status::invalid_argument("price must be a decimal string"))?)
        };
        let cost = if r.cost_price.is_empty() {
            None
        } else {
            Some(r.cost_price.parse::<rust_decimal::Decimal>().map_err(|_| Status::invalid_argument("cost_price must be a decimal string"))?)
        };
        let channel = channel_of(&r.channel).to_string();
        match rustygod_db::channels::set_variant_price(db, &channel, vid, price, cost).await {
            Ok(()) => Ok(Response::new(SetVariantPriceResponse { errors: vec![] })),
            Err(e) => Ok(Response::new(SetVariantPriceResponse {
                errors: vec![Self::ch_err(e)],
            })),
        }
    }
}

#[tonic::async_trait]
impl TaxService for TaxServiceImpl {
    async fn list_tax_classes(
        &self,
        _request: Request<ListTaxClassesRequest>,
    ) -> Result<Response<ListTaxClassesResponse>, Status> {
        let classes = commerce::list_tax_classes(self.db()?)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListTaxClassesResponse {
            classes: classes
                .into_iter()
                .map(|(id, name)| TaxClassInfo {
                    id: id.to_string(),
                    name,
                })
                .collect(),
        }))
    }

    async fn get_tax_rate(
        &self,
        request: Request<GetTaxRateRequest>,
    ) -> Result<Response<GetTaxRateResponse>, Status> {
        let db = self.db()?;
        let r = request.into_inner();
        let channel = channel_of(&r.channel);
        let cfg = rustygod_db::taxes::channel_config(db, channel)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let class = if r.variant_id.is_empty() {
            None
        } else {
            let vid: i32 = r.variant_id.parse().map_err(|_| Status::invalid_argument("variant_id must be an integer"))?;
            Some(
                rustygod_db::taxes::class_for_variant(db, vid)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?,
            )
        };
        let rate = rustygod_db::taxes::rate_for_class(db, class.flatten(), &r.country)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(GetTaxRateResponse {
            rate: rate.normalize().to_string(),
            charge_taxes: cfg.charge_taxes,
            errors: vec![],
        }))
    }

    async fn calculate_taxes(
        &self,
        request: Request<CalculateTaxesRequest>,
    ) -> Result<Response<CalculateTaxesResponse>, Status> {
        let db = self.db()?;
        let r = request.into_inner();
        let channel = channel_of(&r.channel).to_string();
        let mut lines = Vec::with_capacity(r.lines.len());
        for l in &r.lines {
            let vid: i32 = l.variant_id.parse().map_err(|_| Status::invalid_argument("variant_id must be an integer"))?;
            let unit: rust_decimal::Decimal = l.unit_price.parse().map_err(|_| Status::invalid_argument("unit_price must be a decimal string"))?;
            if l.quantity < 0 {
                return Err(Status::invalid_argument("quantity cannot be negative"));
            }
            lines.push((vid, unit, l.quantity));
        }
        let taxed = rustygod_db::taxes::calculate_lines(db, &channel, &r.country, &lines)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let (mut total_net, mut total_gross) =
            (rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO);
        let infos = taxed
            .into_iter()
            .map(|t| {
                total_net += t.total_net;
                total_gross += t.total_gross;
                TaxedLineInfo {
                    variant_id: t.variant_id.to_string(),
                    quantity: t.quantity,
                    unit_net: t.unit_net.to_string(),
                    unit_gross: t.unit_gross.to_string(),
                    total_net: t.total_net.to_string(),
                    total_gross: t.total_gross.to_string(),
                    tax_rate: t.tax_rate.normalize().to_string(),
                }
            })
            .collect();
        Ok(Response::new(CalculateTaxesResponse {
            lines: infos,
            total_net: total_net.to_string(),
            total_gross: total_gross.to_string(),
            errors: vec![],
        }))
    }
}

#[tonic::async_trait]
impl WarehouseService for WarehouseServiceImpl {
    async fn list_warehouses(
        &self,
        request: Request<ListWarehousesRequest>,
    ) -> Result<Response<ListWarehousesResponse>, Status> {
        let req = request.into_inner();
        let wh = commerce::list_warehouses(self.db()?, channel_of(&req.channel))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListWarehousesResponse {
            warehouses: wh
                .into_iter()
                .map(|w| WarehouseInfo {
                    id: w.id,
                    name: w.name,
                    slug: w.slug,
                    email: w.email,
                })
                .collect(),
        }))
    }

    async fn list_stocks(
        &self,
        request: Request<ListStocksRequest>,
    ) -> Result<Response<ListStocksResponse>, Status> {
        let vid: i32 = request.into_inner().variant_id.parse().unwrap_or(-1);
        let stocks = commerce::stocks_for_variant(self.db()?, vid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListStocksResponse {
            stocks: stocks
                .into_iter()
                .map(|s| StockInfo {
                    warehouse_id: s.warehouse_id,
                    quantity: s.quantity,
                    quantity_allocated: s.quantity_allocated,
                })
                .collect(),
        }))
    }

    async fn reserve_stock(
        &self,
        request: Request<ReserveStockRequest>,
    ) -> Result<Response<ReserveStockResponse>, Status> {
        let req = request.into_inner();
        let db = self.db()?;
        let vid: i32 = req.variant_id.parse().unwrap_or(-1);
        let Ok(wh) = req.warehouse_id.parse() else {
            return Ok(Response::new(ReserveStockResponse {
                ok: false,
                errors: vec![err("VALIDATION", "bad warehouse id".into())],
            }));
        };
        let Ok(line) = req.checkout_line_id.parse() else {
            return Ok(Response::new(ReserveStockResponse {
                ok: false,
                errors: vec![err("VALIDATION", "bad checkout line id".into())],
            }));
        };
        match commerce::reserve_stock(db, vid, wh, req.quantity, line, req.expires_in_seconds).await
        {
            Ok(()) => Ok(Response::new(ReserveStockResponse { ok: true, errors: vec![] })),
            Err(e) => Ok(Response::new(ReserveStockResponse {
                ok: false,
                errors: vec![err("INSUFFICIENT_STOCK", e.to_string())],
            })),
        }
    }

    async fn release_reservation(
        &self,
        request: Request<ReleaseReservationRequest>,
    ) -> Result<Response<ReleaseReservationResponse>, Status> {
        let Ok(line) = request.into_inner().checkout_line_id.parse() else {
            return Ok(Response::new(ReleaseReservationResponse { ok: false }));
        };
        commerce::release_reservations(self.db()?, line)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ReleaseReservationResponse { ok: true }))
    }

    async fn create_warehouse(
        &self,
        request: Request<CreateWarehouseRequest>,
    ) -> Result<Response<WarehouseResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PRODUCTS).await?;
        let r = request.into_inner();
        match rustygod_db::warehouses::create_warehouse(
            db,
            rustygod_db::warehouses::NewWarehouse {
                name: r.name,
                slug: r.slug,
                email: r.email,
                street: r.street,
                city: r.city,
                postal_code: r.postal_code,
                country: r.country,
                is_private: r.is_private,
                cc_option: if r.cc_option.is_empty() { "disabled".into() } else { r.cc_option },
            },
        )
        .await
        {
            Ok(w) => Ok(Response::new(WarehouseResponse {
                warehouse: Some(Self::wh_info(&w)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(WarehouseResponse {
                warehouse: None,
                errors: vec![Self::wh_err(e)],
            })),
        }
    }

    async fn update_warehouse(
        &self,
        request: Request<UpdateWarehouseRequest>,
    ) -> Result<Response<WarehouseResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PRODUCTS).await?;
        let r = request.into_inner();
        let id: uuid::Uuid = r.id.parse().map_err(|_| Status::invalid_argument("id must be a UUID"))?;
        match rustygod_db::warehouses::update_warehouse(
            db,
            id,
            rustygod_db::warehouses::WarehousePatch {
                name: (!r.name.is_empty()).then_some(r.name),
                email: (!r.email.is_empty()).then_some(r.email),
                cc_option: (!r.cc_option.is_empty()).then_some(r.cc_option),
                is_private: r.set_private.then_some(r.is_private),
            },
        )
        .await
        {
            Ok(w) => Ok(Response::new(WarehouseResponse {
                warehouse: Some(Self::wh_info(&w)),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(WarehouseResponse {
                warehouse: None,
                errors: vec![Self::wh_err(e)],
            })),
        }
    }

    async fn delete_warehouse(
        &self,
        request: Request<DeleteWarehouseRequest>,
    ) -> Result<Response<DeleteWarehouseResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PRODUCTS).await?;
        let id: uuid::Uuid = request.into_inner().id.parse().map_err(|_| Status::invalid_argument("id must be a UUID"))?;
        match rustygod_db::warehouses::delete_warehouse(db, id).await {
            Ok(()) => Ok(Response::new(DeleteWarehouseResponse { ok: true, errors: vec![] })),
            Err(e) => Ok(Response::new(DeleteWarehouseResponse {
                ok: false,
                errors: vec![Self::wh_err(e)],
            })),
        }
    }

    async fn upsert_stock(
        &self,
        request: Request<UpsertStockRequest>,
    ) -> Result<Response<UpsertStockResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PRODUCTS).await?;
        let r = request.into_inner();
        let wh: uuid::Uuid = r.warehouse_id.parse().map_err(|_| Status::invalid_argument("warehouse_id must be a UUID"))?;
        let vid: i32 = r.variant_id.parse().map_err(|_| Status::invalid_argument("variant_id must be an integer"))?;
        match rustygod_db::warehouses::upsert_stock(db, wh, vid, r.quantity).await {
            Ok(s) => Ok(Response::new(UpsertStockResponse {
                stock: Some(StockInfo {
                    warehouse_id: s.warehouse_id.to_string(),
                    quantity: s.quantity,
                    quantity_allocated: s.quantity_allocated,
                }),
                errors: vec![],
            })),
            Err(e) => Ok(Response::new(UpsertStockResponse {
                stock: None,
                errors: vec![Self::wh_err(e)],
            })),
        }
    }

    async fn list_zones(
        &self,
        _request: Request<ListZonesRequest>,
    ) -> Result<Response<ListZonesResponse>, Status> {
        let zones = rustygod_db::warehouses::list_zones(self.db()?)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListZonesResponse {
            zones: zones
                .into_iter()
                .map(|z| ZoneInfo {
                    id: z.id.to_string(),
                    name: z.name,
                    countries: z.countries,
                    is_default: z.is_default,
                })
                .collect(),
        }))
    }

    async fn assign_zone(
        &self,
        request: Request<ZoneLinkRequest>,
    ) -> Result<Response<ZoneLinkResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PRODUCTS).await?;
        let r = request.into_inner();
        let wh: uuid::Uuid = r.warehouse_id.parse().map_err(|_| Status::invalid_argument("warehouse_id must be a UUID"))?;
        let zid: i32 = r.zone_id.parse().map_err(|_| Status::invalid_argument("zone_id must be an integer"))?;
        match rustygod_db::warehouses::assign_zone(db, wh, zid).await {
            Ok(()) => Ok(Response::new(ZoneLinkResponse { ok: true, errors: vec![] })),
            Err(e) => Ok(Response::new(ZoneLinkResponse {
                ok: false,
                errors: vec![Self::wh_err(e)],
            })),
        }
    }

    async fn unassign_zone(
        &self,
        request: Request<ZoneLinkRequest>,
    ) -> Result<Response<ZoneLinkResponse>, Status> {
        let db = self.db()?;
        crate::access::authorize(db, request.metadata(), crate::access::MANAGE_PRODUCTS).await?;
        let r = request.into_inner();
        let wh: uuid::Uuid = r.warehouse_id.parse().map_err(|_| Status::invalid_argument("warehouse_id must be a UUID"))?;
        let zid: i32 = r.zone_id.parse().map_err(|_| Status::invalid_argument("zone_id must be an integer"))?;
        match rustygod_db::warehouses::unassign_zone(db, wh, zid).await {
            Ok(ok) => Ok(Response::new(ZoneLinkResponse { ok, errors: vec![] })),
            Err(e) => Ok(Response::new(ZoneLinkResponse {
                ok: false,
                errors: vec![Self::wh_err(e)],
            })),
        }
    }
}

pub fn _offline<T>() -> Result<Response<T>, Status> {
    unavailable()
}

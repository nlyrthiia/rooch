
module minter_manager::usdt_m {
    use std::option::some;
    use std::string;
    use std::string::utf8;
    use moveos_std::signer::module_signer;
    use minter_manager::minter_manager::setupTreasuryCapManager;
    use rooch_framework::coin;

    struct USDT has key, store{}

    const COIN_URL: vector<u8> = b"<svg id=\"Layer_1\" data-name=\"Layer 1\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 339.43 295.27\"><title>tether-usdt-logo</title><path d=\"M62.15,1.45l-61.89,130a2.52,2.52,0,0,0,.54,2.94L167.95,294.56a2.55,2.55,0,0,0,3.53,0L338.63,134.4a2.52,2.52,0,0,0,.54-2.94l-61.89-130A2.5,2.5,0,0,0,275,0H64.45a2.5,2.5,0,0,0-2.3,1.45h0Z\" style=\"fill:#50af95;fill-rule:evenodd\"/><path d=\"M191.19,144.8v0c-1.2.09-7.4,0.46-21.23,0.46-11,0-18.81-.33-21.55-0.46v0c-42.51-1.87-74.24-9.27-74.24-18.13s31.73-16.25,74.24-18.15v28.91c2.78,0.2,10.74.67,21.74,0.67,13.2,0,19.81-.55,21-0.66v-28.9c42.42,1.89,74.08,9.29,74.08,18.13s-31.65,16.24-74.08,18.12h0Zm0-39.25V79.68h59.2V40.23H89.21V79.68H148.4v25.86c-48.11,2.21-84.29,11.74-84.29,23.16s36.18,20.94,84.29,23.16v82.9h42.78V151.83c48-2.21,84.12-11.73,84.12-23.14s-36.09-20.93-84.12-23.15h0Zm0,0h0Z\" style=\"fill:#fff;fill-rule:evenodd\"/></svg>";

    fun init() {
        let coin_info_obj = coin::register_extend<USDT>(
            string::utf8(b"USDT From Mesion Fi"),
            string::utf8(b"USDT.M"),
            some(utf8(COIN_URL)),
            6,
        );
        setupTreasuryCapManager(&module_signer<USDT>(), coin_info_obj)
    }
}

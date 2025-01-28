import { SecretNetworkClient, Wallet } from "secretjs";
import dotenv from "dotenv";
dotenv.config();

const wallet = new Wallet(process.env.MNEMONIC);

const secretjs = new SecretNetworkClient({
    chainId: "pulsar-3",
    url: "https://api.pulsar.scrttestnet.com",
    wallet: wallet,
    walletAddress: wallet.address,
});

let queryContractInfo = async () => {
    let query = await secretjs.query.compute.contractInfo({
        contract_address: "secret1dzynxw5hvcy5tm0mg4k2ftaqwuzfyce2ydjzj4",
        code_hash: "afd0b5bda5a14dd41dc98d4cf112c1a239b5689796ac0fec4845db69d0a11f28",
    });

    console.log(query);
};

queryContractInfo();
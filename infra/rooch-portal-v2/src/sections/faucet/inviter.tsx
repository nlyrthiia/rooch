'use client';

import type { Bytes} from '@roochnetwork/rooch-sdk';

import { useState, useEffect } from 'react';
import { Args, toHEX, stringToBytes } from '@roochnetwork/rooch-sdk';
import {
  useRoochClient,
  useCurrentWallet,
  useCurrentAddress,
  useRoochClientQuery,
} from '@roochnetwork/rooch-sdk-kit';

import { LoadingButton } from '@mui/lab';
import { Box, Card, Chip, Stack, CardHeader, CardContent } from '@mui/material';

import { useRouter } from 'src/routes/hooks';

import { useNetworkVariable } from 'src/hooks/use-networks';

import { formatCoin } from 'src/utils/format-number';
import { INVITER_ADDRESS_KEY } from 'src/utils/inviter';

import { DashboardContent } from 'src/layouts/dashboard';

import { toast } from 'src/components/snackbar';

import { paths } from '../../routes/paths';

const FAUCET_NOT_OPEN = 'Faucet Not Open';
const INVALID_UTXO = 'Invalid UTXO';
const FAUCET_NOT_ENOUGH_RGAS = 'Faucet Not enough RGas';
const ALREADY_CLAIMED = 'Already Claimed';
const UTXO_VALUE_IS_ZERO = 'UTXO Value Is Zero';

const ERROR_MSG: Record<string, string> = {
  1: FAUCET_NOT_OPEN,
  2: INVALID_UTXO,
  3: FAUCET_NOT_ENOUGH_RGAS,
  4: ALREADY_CLAIMED,
  5: UTXO_VALUE_IS_ZERO,
};

export function InviterFaucetView({ inviterAddress }: { inviterAddress: string }) {
  const router = useRouter();

  const client = useRoochClient();
  const faucetAddress = useNetworkVariable('faucetAddress');
  const faucetObject = useNetworkVariable('faucetObject');
  const [inviterCA, inviterName] = useNetworkVariable('inviterCA')
  const inviterConf = `${inviterCA}::${inviterName}::InvitationConf`;
  const faucetUrl = useNetworkVariable('faucetUrl');
  const wallet = useCurrentWallet();

  const viewAddress = useCurrentAddress();
  const [faucetStatus, setFaucetStatus] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string>();
  const [claimGas, setClaimGas] = useState(0);
  const [UTXOs, setUTXOs] = useState<Array<string> | null>(null);

  const { data: inviter } = useRoochClientQuery('queryObjectStates', {
    filter: {
      object_type: inviterConf,
    },
    queryOption: {
      decode: true,
    },
  });

  useEffect(() => {

    // invite close
    if (inviter && inviter.data.length > 0 && inviter.data[0].decoded_value?.value.is_open === false) {
      router.push(paths.dashboard.faucet);
    }

  }, [inviter, router])

  const { data, isPending, refetch } = useRoochClientQuery(
    'getBalance',
    {
      owner: viewAddress?.genRoochAddress().toStr()!,
      coinType: '0x3::gas_coin::RGas',
    },
    { refetchInterval: 5000 }
  );

  useEffect(() => {
    if (!viewAddress) {
      return;
    }
    setFaucetStatus(true);
    client
      .queryUTXO({
        filter: {
          owner: viewAddress.toStr(),
        },
      })
      .then(async (result) => {
        const utxoIds = result.data.map((item) => item.id);
        if (utxoIds) {
          setUTXOs(utxoIds);
          const result = await client.executeViewFunction({
            target: `${faucetAddress}::gas_faucet::check_claim`,
            args: [
              Args.objectId(faucetObject),
              Args.address(viewAddress.genRoochAddress()!),
              Args.vec('objectId', utxoIds),
            ],
          });

          if (result.vm_status === 'Executed') {
            const gas = Number(formatCoin(Number(result.return_values![0].decoded_value), 8, 2));
            setClaimGas(gas);
          } else if ('MoveAbort' in result.vm_status) {
            setErrorMsg(ERROR_MSG[Number(result.vm_status.MoveAbort.abort_code)]);
          }
        } else {
          setErrorMsg('Not found utxo');
        }
      })
      .finally(() => {
        setFaucetStatus(false);
      });
  }, [client, faucetAddress, faucetObject, viewAddress]);

  const fetchFaucet = async () => {
    if (errorMsg === ALREADY_CLAIMED) {
      router.push(paths.dashboard['gas-swap']);
      return;
    }

    setFaucetStatus(true);

    if (
      inviterAddress &&
      inviter &&
      inviter.data.length > 0 &&
      inviter.data[0].decoded_value?.value.is_open === true
    ) {
      let sign: Bytes | undefined
      const pk = wallet.wallet!.getPublicKey().toBytes()
      const signMsg = 'Welcome to use Rooch! Hold BTC Claim your Rgas.'
      try {
        sign = await wallet.wallet?.sign(stringToBytes('utf8', signMsg))
      } catch (e) {
        toast.error(e.message)
      }

      if (!sign) {
        return;
      }

      try {
        const payload = JSON.stringify({
          claimer: viewAddress!.toStr(),
          inviter: inviterAddress,
          claimer_sign: toHEX(sign),
          public_key: toHEX(pk),
          message: signMsg,
        });
        const response = await fetch(`${faucetUrl}/faucet-inviter`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: payload,
        });

        if (!response.ok) {
          const data = await response.json();
          if (response.status === 500 && data.error.includes('UTXO value is zero')) {
            const msg = 'Claim failed, Not found UTXO';
            setErrorMsg(msg);
            toast.error(msg);
            return;
          }

          toast.error('Network response was not ok');
          return;
        }

        const d = await response.json();
        window.localStorage.setItem(INVITER_ADDRESS_KEY, '')
        await refetch();
        toast.success(
          `Faucet Success! RGas: ${formatCoin(Number(d.gas || 0), data?.decimals || 0, 2)}`
        );
      } catch (error) {
        console.error('Error:', error);
        toast.error(`faucet error: ${error}`);
      } finally {
        setFaucetStatus(false);
      }
    }
  };

  return (
    <DashboardContent maxWidth="xl">
      <Card>
        <CardHeader title="Gas Faucet" sx={{ mb: 1 }} />
        <CardContent className="!pt-0">
          <Stack spacing={2}>
            <Stack direction="row" alignItems="center" spacing={0.5}>
              <Chip className="w-fit" label="Claim Address:" variant="soft" color="default" />
              <Box className="text-gray-400 text-sm font-medium">({viewAddress?.toStr()})</Box>
            </Stack>
            <Stack direction="row" alignItems="center" spacing={0.5}>
              <Chip className="w-fit" label="Claim Rooch Address:" variant="soft" color="default" />
              <Box className="text-gray-400 text-sm font-medium">
                ({viewAddress?.genRoochAddress().toStr()})
              </Box>
            </Stack>
            <Stack direction="row" alignItems="center" spacing={0.5}>
              <Chip className="w-fit" label="RGas Balance:" variant="soft" color="secondary" />
              <Box className="text-gray-400 text-sm font-medium">
                {formatCoin(Number(data?.balance || 0), data?.decimals || 0, 2)}
              </Box>
            </Stack>
            {errorMsg
              ? errorMsg === ALREADY_CLAIMED
                ? 'You Already Claimed RGAS'
                : 'You cannot claim gas, Please make sure the current address has a valid utxo and try again'
              : ''}
              <LoadingButton
                variant="soft"
                color="primary"
                disabled={errorMsg !== undefined && errorMsg !== ALREADY_CLAIMED}
                loading={isPending || faucetStatus}
                onClick={fetchFaucet}
              >
                {errorMsg === ALREADY_CLAIMED
                  ? 'Purchase RGas'
                  : errorMsg || `Claim: ${claimGas} RGas`}
              </LoadingButton>
          </Stack>
        </CardContent>
      </Card>
    </DashboardContent>
  );
}

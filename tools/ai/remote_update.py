"""Run one PPO iteration's optimizer update in another process, perhaps on
another machine, while self-play stays where the training run lives.

``fleet_ppo.py --update-cmd CMD`` pipes each iteration's samples, the
weights, the KL reference, the optimizer state and the random state to
``CMD``'s standard input, and reads the updated weights, optimizer and
random state back from its standard output. ``CMD`` runs this file, e.g.
``ssh crunch sh nc-ai-state/t235/update.sh``. The worker keeps no state
between iterations, so the training run alone holds ``resume.pt`` and a
worker that fails or cannot be reached, as while its machine reboots,
leaves that iteration's update to the training run itself.

Usage (the worker): python remote_update.py [--expect ARCH:BUS]

``--expect gfx1100:3`` refuses to start unless the one visible GPU is that
architecture on that PCI bus.
"""
import argparse
import io
import subprocess
import sys
import time
import zlib

import torch


def update_remotely(cmd, model, ref, optimizer, steps, advs, coefs, args, upgo):
    """``fleet_ppo.update``'s result, computed by the worker ``cmd``, which
    leaves ``model``, ``optimizer`` and the torch random state as an
    in-process update would; or None, changing nothing, if the worker
    fails, so the caller can update in-process instead."""
    payload = io.BytesIO()
    torch.save(dict(config=model.config, model=model.state_dict(), ref=ref.state_dict(),
                    optimizer=optimizer.state_dict(), steps=steps, advs=advs, upgo=upgo,
                    coefs=coefs, args=args, torch_rng=torch.get_rng_state()), payload)
    # The padded samples shrink many times over, and the link is slower than zlib.
    done = subprocess.run(cmd, shell=True, input=zlib.compress(payload.getvalue(), 1),
                          capture_output=True)
    if done.returncode:
        print(f"update worker failed ({done.returncode}), updating here: "
              f"{done.stderr.decode(errors='replace')[-500:]}", file=sys.stderr, flush=True)
        return None
    sys.stderr.write(done.stderr.decode(errors="replace"))
    result = torch.load(io.BytesIO(done.stdout), map_location="cpu", weights_only=False)
    model.load_state_dict(result["model"])
    optimizer.load_state_dict(result["optimizer"])
    torch.set_rng_state(result["torch_rng"])
    return result["stats"], result["samples"]


def check_gpu(expect):
    arch, bus = expect.split(":")
    if torch.cuda.device_count() != 1:
        raise SystemExit(f"expected one visible GPU, found {torch.cuda.device_count()}")
    props = torch.cuda.get_device_properties(0)
    if not props.gcnArchName.startswith(arch) or props.pci_bus_id != int(bus):
        raise SystemExit(f"wrong GPU: {props.name} {props.gcnArchName} bus {props.pci_bus_id}")
    print(f"update worker on {props.name} {props.gcnArchName} bus {props.pci_bus_id}",
          file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--expect", help="ARCH:BUS the one visible GPU must be")
    cli = parser.parse_args()
    torch.set_num_threads(1)
    torch.set_num_interop_threads(1)
    if cli.expect:
        check_gpu(cli.expect)
    # Imported after the thread limits, so nothing starts a wider pool first.
    from fleet_policy import FleetAgent
    from fleet_ppo import update
    from policy import device

    dev = device()
    started = time.perf_counter()
    data = sys.stdin.buffer.read()
    job = torch.load(io.BytesIO(zlib.decompress(data)), map_location="cpu", weights_only=False)
    loaded = time.perf_counter()
    model, ref = FleetAgent(**job["config"]).to(dev), FleetAgent(**job["config"]).to(dev)
    model.load_state_dict(job["model"])
    ref.load_state_dict(job["ref"])
    optimizer = torch.optim.AdamW(model.parameters(), lr=job["args"].lr)
    optimizer.load_state_dict(job["optimizer"])
    torch.set_rng_state(job["torch_rng"])
    stats, samples = update(model, ref, optimizer, job["steps"], job["advs"], *job["coefs"],
                            job["args"], dev, job["upgo"])
    print(f"update worker: {len(data) / 1e6:.0f} MB in {loaded - started:.1f}s, "
          f"update {time.perf_counter() - loaded:.1f}s", file=sys.stderr)
    out = io.BytesIO()
    torch.save(dict(model=model.state_dict(), optimizer=optimizer.state_dict(), stats=stats,
                    samples=samples, torch_rng=torch.get_rng_state()), out)
    sys.stdout.buffer.write(out.getvalue())


if __name__ == "__main__":
    main()

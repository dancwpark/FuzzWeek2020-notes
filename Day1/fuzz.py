import os
import glob
import subprocess
import random
import time
import threading

# Run one fuzz case with the provided input (which is a byte array)
def fuzz(thr_id: int, inp: bytearray): # can type... but it's not checked...?
    assert isinstance(thr_id, int)
    # Same
    assert isinstance(inp, bytearray)  
    assert type(inp) == bytearray
   
    # Write out the input to a temporary file 
    tmpfn = f"tmpinput{thr_id}"
    with open(tmpfn, "wb") as fd:
        fd.write(inp)

    # Run objdump until completion
    sp = subprocess.Popen(["./objdump", "-x", tmpfn], 
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL)
    ret = sp.wait()
    # print(ret) # prints return code

    # Assert that the program ran successfully
    # assert ret >= 0 # crashes use negative numbers
    if ret != 0:
        print(f"Exited with {ret}")

     

# Get a listing of all the files in the corpus
# The corpus is the set of files which we pre-seeded the fuzzers with
#  to give it valid input. These are files taht the program should be
#  able to handle parsing, taht we will ultimately mutate and plice together
#  to try to find bugs!
corpus_filenames = os.listdir("corpus")
corpus_filenames = glob.glob("corpus/*") # glob is better b/c full paths
print(corpus_filenames)

# Load the corpus files into memory
corpus = set() # using set to get rid of aliases/symlinks/dups
for filename in corpus_filenames:
    corpus.add(open(filename, "rb").read())

# Convert the corpus back into a list as we're done with the set for
# deduping inputs which were not unique
corpus = list(map(bytearray, corpus)) # bytearray for in-place mutations

# Get the time at the start of the fuzzer
start = time.time()

# Total number of fuzz cases
cases = 0

def worker(thr_id):
    global start, corpus, cases

    print("worker")
    
    while True:
        # Create a copy of a random existing input from the corpus
        inp = bytearray(random.choice(corpus))
        
        # Mutate it
        for _ in range(random.randint(1, 8)):
            inp[random.randint(0, len(inp))] = random.randint(0, 255)
        
        # Fuzz the mutated input
        fuzz(thr_id, inp)
        
        # Update number of fuzz cases
        cases += 1

        # determine the amount of seconds we have been fuzzing for
        elapsed = time.time() - start
    
        # determine the number of fuzz cases per second
        fcps = float(cases) / elapsed
    
        print(f"[{elapsed:10.4f}] cases {cases:10} | fcps {fcps:10.4f}")
    
for thr_id in range(5):    
    threading.Thread(target=worker, args=[thr_id]).start()

while threading.active_count() > 0:
    time.sleep(0.1)

    
